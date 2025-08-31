use std::{
    ffi::c_int,
    sync::atomic::{AtomicI64, Ordering},
    time::Duration,
};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, TryStreamExt,
};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use tokio::{net::TcpStream, sync::mpsc, time::timeout};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        http::response,
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message, Utf8Bytes,
    },
    MaybeTlsStream, WebSocketStream,
};

use lib_core::context::CONTEXT;
use lib_lua::{
    self, cstr,
    ffi::{self, luaL_Reg},
    laux::{self, lua_into_userdata, LuaState, LuaTable},
    lreg, lreg_null, luaL_newlib,
};

use crate::moon_send;

lazy_static! {
    static ref NET: DashMap<i64, WsChannel> = DashMap::new();
    static ref NET_UUID: AtomicI64 = AtomicI64::new(1);
}

#[derive(Clone)]
struct WsChannel {
    pub tx_reader: mpsc::Sender<WsRequest>,
    pub tx_writer: mpsc::Sender<WsRequest>,
}

#[derive(Debug)]
enum WsRequest {
    Read(u32, i64, u64),  // owner, session, read_timeout
    Write(Message, bool), // owner, session, data, close
    Close(Message),
}

enum WsResponse {
    Connect(i64, response::Response<Option<Vec<u8>>>),
    Read(Message),
    Error(anyhow::Error),
}

fn next_net_fd() -> i64 {
    let fd = NET_UUID.fetch_add(1, Ordering::AcqRel);
    if fd == i64::MAX {
        panic!("net fd overflow");
    }
    fd
}

async fn handle_read(
    protocol_type: u8,
    owner: u32,
    session: i64,
    read_timeout: u64,
    reader: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Result<()> {
    if read_timeout > 0 {
        if let Some(message) =
            timeout(Duration::from_millis(read_timeout), reader.try_next()).await??
        {
            moon_send(protocol_type, owner, session, WsResponse::Read(message));
            Ok(())
        } else {
            Err(anyhow!("eof"))
        }
    } else if let Some(message) = reader.try_next().await? {
        moon_send(protocol_type, owner, session, WsResponse::Read(message));
        Ok(())
    } else {
        Err(anyhow!("eof"))
    }
}

async fn handle_write(
    mut writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut rx: mpsc::Receiver<WsRequest>,
) -> Result<()> {
    while let Some(op) = rx.recv().await {
        match op {
            WsRequest::Write(data, close) => {
                writer.send(data).await?;
                if close {
                    return Ok(());
                }
            }
            WsRequest::Close(data) => {
                writer.send(data).await?;
                return Ok(());
            }
            _ => {
                log::error!("write: {:?}", op);
            }
        }
    }
    Err(anyhow!("writer closed"))
}

async fn handle_client(
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    response: response::Response<Option<Vec<u8>>>,
    protocol_type: u8,
    owner: u32,
    session: i64,
) {
    let fd = next_net_fd();
    let (tx_reader, mut rx_reader) = mpsc::channel::<WsRequest>(1);
    let (tx_writer, rx_writer) = mpsc::channel::<WsRequest>(100);
    NET.insert(
        fd,
        WsChannel {
            tx_reader,
            tx_writer,
        },
    );

    moon_send(
        protocol_type,
        owner,
        session,
        WsResponse::Connect(fd, response),
    );

    let (writer, mut reader) = stream.split();

    let mut read_task = CONTEXT.tokio_runtime.spawn(async move {
        let mut closed = false;
        while let Some(op) = rx_reader.recv().await {
            if let WsRequest::Read(owner, session, read_timeout) = op {
                if !closed {
                    if let Err(err) =
                        handle_read(protocol_type, owner, session, read_timeout, &mut reader).await
                    {
                        moon_send(protocol_type, owner, session, WsResponse::Error(err));
                        closed = true;
                    }
                } else {
                    moon_send(
                        protocol_type,
                        owner,
                        session,
                        WsResponse::Error(anyhow!("closed")),
                    );
                }
            }
        }
    });

    let mut write_task = CONTEXT.tokio_runtime.spawn(handle_write(writer, rx_writer));

    if tokio::try_join!(&mut read_task, &mut write_task).is_err() {
        read_task.abort();
        write_task.abort();
    };
}

extern "C-unwind" fn lread(state: LuaState) -> c_int {
    let conn =
        laux::lua_touserdata::<WsChannel>(state, 1).expect("Invalid database connect pointer");
    let owner = laux::lua_get(state, 2);
    let session = laux::lua_get(state, 3);
    let timeout = laux::lua_opt(state, 4).unwrap_or(5000);

    match conn
        .tx_reader
        .try_send(WsRequest::Read(owner, session, timeout))
    {
        Ok(_) => {
            laux::lua_push(state, session);
            1
        }
        Err(err) => {
            laux::lua_push(state, false);
            laux::lua_push(state, format!("read error: {}", err));
            2
        }
    }
}

extern "C-unwind" fn lwrite(state: LuaState) -> c_int {
    let conn =
        laux::lua_touserdata::<WsChannel>(state, 1).expect("Invalid database connect pointer");

    let data: &[u8] = laux::lua_get(state, 2);
    let c: &str = laux::lua_opt(state, 3).unwrap_or("b");
    let mut close = false;

    let m = match c {
        "c" => {
            close = true;
            Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: unsafe { Utf8Bytes::from_bytes_unchecked(data.into()) },
            }))
        }
        "t" => Message::Text(unsafe { Utf8Bytes::from_bytes_unchecked(data.into()) }),
        "p" => {
            close = true;
            Message::Ping(data.into())
        }
        _ => Message::Binary(data.into()),
    };

    match conn.tx_writer.try_send(WsRequest::Write(m, close)) {
        Ok(_) => {
            laux::lua_push(state, true);
            1
        }
        Err(err) => {
            laux::lua_push(state, false);
            laux::lua_push(state, format!("read error: {}", err));
            2
        }
    }
}

extern "C-unwind" fn lclose(state: LuaState) -> c_int {
    let conn =
        laux::lua_touserdata::<WsChannel>(state, 1).expect("Invalid database connect pointer");

    let data: &[u8] = laux::lua_opt(state, 2).unwrap_or_default();

    match conn
        .tx_writer
        .try_send(WsRequest::Close(Message::Close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: unsafe { Utf8Bytes::from_bytes_unchecked(data.into()) },
        })))) {
        Ok(_) => {
            laux::lua_push(state, true);
            1
        }
        Err(err) => {
            laux::lua_push(state, false);
            laux::lua_push(state, format!("close error: {}", err));
            2
        }
    }
}

extern "C-unwind" fn lconnect(state: LuaState) -> c_int {
    let protocol_type: u8 = laux::opt_field(state, 1, "protocol_type").unwrap_or(0);
    let session = laux::opt_field(state, 1, "session").unwrap_or(0);
    let owner = laux::opt_field(state, 1, "owner").unwrap_or_default();
    let url: String = laux::opt_field(state, 1, "url").unwrap_or_default();
    let connect_timeout = laux::opt_field(state, 1, "connect_timeout").unwrap_or(5000);

    CONTEXT.tokio_runtime.spawn(async move {
        match timeout(Duration::from_millis(connect_timeout), connect_async(url)).await {
            Ok(Ok(stream)) => {
                CONTEXT.tokio_runtime.spawn(async move {
                    handle_client(stream.0, stream.1, protocol_type, owner, session).await;
                });
            }
            Ok(Err(err)) => {
                moon_send(protocol_type, owner, session, WsResponse::Error(err.into()));
            }
            Err(err) => {
                moon_send(protocol_type, owner, session, WsResponse::Error(err.into()));
            }
        }
    });

    laux::lua_push(state, session);

    1
}

extern "C-unwind" fn find_connection(state: LuaState) -> c_int {
    let id: i64 = laux::lua_get(state, 1);
    match NET.get(&id) {
        Some(pair) => {
            let l = [
                lreg!("read", lread),
                lreg!("write", lwrite),
                lreg!("close", lclose),
                lreg_null!(),
            ];
            if laux::lua_newuserdata(
                state,
                pair.value().clone(),
                cstr!("ws_connection_metatable"),
                l.as_ref(),
            )
            .is_none()
            {
                laux::lua_pushnil(state);
                return 1;
            }
        }
        None => {
            laux::lua_pushnil(state);
        }
    }
    1
}

fn version_to_string(version: &reqwest::Version) -> &str {
    match *version {
        reqwest::Version::HTTP_09 => "HTTP/0.9",
        reqwest::Version::HTTP_10 => "HTTP/1.0",
        reqwest::Version::HTTP_11 => "HTTP/1.1",
        reqwest::Version::HTTP_2 => "HTTP/2.0",
        reqwest::Version::HTTP_3 => "HTTP/3.0",
        _ => "Unknown",
    }
}

extern "C-unwind" fn decode(state: LuaState) -> c_int {
    laux::luaL_checkstack(state, 6, std::ptr::null());
    let response = lua_into_userdata::<WsResponse>(state, 1);
    match *response {
        WsResponse::Connect(id, response) => {
            LuaTable::new(state, 0, 6)
                .rawset("fd", id)
                .rawset("version", version_to_string(&response.version()))
                .rawset("status_code", response.status().as_u16())
                .rawset("body", response.body().as_deref().unwrap_or_default())
                .rawset_x("headers", || {
                    let headers = LuaTable::new(state, 0, response.headers().len());
                    for (key, value) in response.headers().iter() {
                        headers.rawset(key.as_str(), value.to_str().unwrap_or("").trim());
                    }
                });
            1
        }
        WsResponse::Read(data) => {
            match data {
                Message::Text(bytes) => {
                    laux::lua_push(state, bytes.as_bytes());
                    laux::lua_push(state, "t");
                }
                Message::Binary(bytes) => {
                    laux::lua_push(state, bytes.as_ref());
                    laux::lua_push(state, "b");
                }
                Message::Ping(bytes) => {
                    laux::lua_push(state, bytes.as_ref());
                    laux::lua_push(state, "p");
                }
                Message::Pong(bytes) => {
                    laux::lua_push(state, bytes.as_ref());
                    laux::lua_push(state, "q");
                }
                Message::Close(bytes) => {
                    laux::lua_push(
                        state,
                        bytes
                            .unwrap_or(CloseFrame {
                                code: CloseCode::Away,
                                reason: "".into(),
                            })
                            .to_string(),
                    );
                    laux::lua_push(state, "c");
                }
                Message::Frame(_) => {
                    laux::lua_push(state, "Raw frame. Note, that you're not going to get this value while reading the message.");
                    laux::lua_push(state, "f");
                }
            }
            2
        }
        WsResponse::Error(error) => {
            laux::lua_push(state, false);
            laux::lua_push(state, error.to_string());
            2
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C-unwind" fn luaopen_rust_websocket(state: LuaState) -> c_int {
    let l = [
        lreg!("connect", lconnect),
        lreg!("find_connection", find_connection),
        lreg!("decode", decode),
        lreg_null!(),
    ];

    luaL_newlib!(state, l);
    1
}
