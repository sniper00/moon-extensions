use dashmap::DashMap;
use lazy_static::lazy_static;
use lib_core::context::CONTEXT;
use lib_lua::{
    self, cstr,
    ffi::{self, luaL_Reg},
    laux::{self, LuaState, LuaTable, LuaValue},
    lreg, lreg_null, luaL_newlib,
};
use reqwest::{header::HeaderMap, ClientBuilder, Method, Version};
use std::time::Duration;
use std::{error::Error, ffi::c_int, str::FromStr};
use url::form_urlencoded::{self};

use crate::moon_send;

lazy_static! {
    static ref HTTP_CLIENTS: DashMap<String, reqwest::Client> = DashMap::new();
}

fn get_http_client(timeout: u64, proxy: &String) -> reqwest::Client {
    let name = format!("{}_{}", timeout, proxy);
    if let Some(client) = HTTP_CLIENTS.get(&name) {
        return client.clone();
    }

    let builder = ClientBuilder::new()
        .timeout(Duration::from_secs(timeout))
        .use_rustls_tls()
        .tcp_nodelay(true);

    let client = if proxy.is_empty() {
        builder.build().unwrap_or_default()
    } else {
        match reqwest::Proxy::all(proxy) {
            Ok(proxy) => builder.proxy(proxy).build().unwrap_or_default(),
            Err(_) => builder.build().unwrap_or_default(),
        }
    };

    HTTP_CLIENTS.insert(name.to_string(), client.clone());
    client
}

struct HttpRequest {
    owner: u32,
    session: i64,
    method: String,
    url: String,
    body: String,
    headers: HeaderMap,
    timeout: u64,
    proxy: String,
}

struct HttpResponse {
    version: Version,
    status_code: i32,
    headers: HeaderMap,
    body: bytes::Bytes,
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

async fn http_request(req: HttpRequest, protocol_type: u8) -> Result<(), Box<dyn Error>> {
    let http_client = get_http_client(req.timeout, &req.proxy);

    let response = http_client
        .request(Method::from_str(req.method.as_str())?, req.url)
        .headers(req.headers)
        .body(req.body)
        .send()
        .await?;

    let response = HttpResponse {
        version: response.version(),
        status_code: response.status().as_u16() as i32,
        headers: response.headers().clone(),
        body: response.bytes().await?,
    };

    moon_send(protocol_type, req.owner, req.session, response);

    Ok(())
}

fn extract_headers(state: LuaState, index: i32) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::with_capacity(8); // Pre-allocate reasonable size

    let table = LuaTable::from_stack(state, index);
    let header_table = table.rawget("headers");

    match &header_table.value {
        LuaValue::Table(header_table) => {
            header_table
                .iter()
                .try_for_each(|(key, value)| {
                    let key_str = key.to_string();
                    let value_str = value.to_string();

                    // Parse header name and value
                    let name = key_str
                        .parse::<reqwest::header::HeaderName>()
                        .map_err(|e| format!("Invalid header name '{}': {}", key_str, e))?;

                    let value = value_str
                        .parse::<reqwest::header::HeaderValue>()
                        .map_err(|e| format!("Invalid header value '{}': {}", value_str, e))?;

                    headers.insert(name, value);
                    Ok(())
                })
                .map_err(|e: String| e)?;
        }
        _ => return Ok(headers), // Empty headers if not a table
    }

    Ok(headers)
}

extern "C-unwind" fn lua_http_request(state: LuaState) -> c_int {
    laux::lua_checktype(state, 1, ffi::LUA_TTABLE);

    let protocol_type = laux::lua_get::<u8>(state, 2);

    let headers = match extract_headers(state, 1) {
        Ok(headers) => headers,
        Err(err) => {
            laux::lua_push(state, false);
            laux::lua_push(state, err);
            return 2;
        }
    };

    let session = laux::opt_field(state, 1, "session").unwrap_or(0);

    let req = HttpRequest {
        owner: laux::opt_field(state, 1, "owner").unwrap_or_default(),
        session,
        method: laux::opt_field(state, 1, "method").unwrap_or("GET".to_string()),
        url: laux::opt_field(state, 1, "url").unwrap_or_default(),
        body: laux::opt_field(state, 1, "body").unwrap_or_default(),
        headers,
        timeout: laux::opt_field(state, 1, "timeout").unwrap_or(5),
        proxy: laux::opt_field(state, 1, "proxy").unwrap_or_default(),
    };

    CONTEXT.tokio_runtime.spawn(async move {
        let session = req.session;
        let owner = req.owner;
        if let Err(err) = http_request(req, protocol_type).await {
            let response = HttpResponse {
                version: Version::HTTP_11,
                status_code: -1,
                headers: HeaderMap::new(),
                body: err.to_string().into(),
            };
            moon_send(protocol_type, owner, session, response);
        }
    });

    laux::lua_push(state, session);
    1
}

extern "C-unwind" fn decode(state: LuaState) -> c_int {
    laux::luaL_checkstack(state, 4, std::ptr::null());
    let p_as_isize: isize = laux::lua_get(state, 1);
    let response = unsafe { Box::from_raw(p_as_isize as *mut HttpResponse) };

    LuaTable::new(state, 0, 6)
        .rawset("version", version_to_string(&response.version))
        .rawset("status_code", response.status_code)
        .rawset("body", response.body.as_ref())
        .rawset_x("headers", || {
            let headers = LuaTable::new(state, 0, response.headers.len());
            for (key, value) in response.headers.iter() {
                headers.rawset(key.as_str(), value.to_str().unwrap_or("").trim());
            }
        });
    1
}

extern "C-unwind" fn lua_http_form_urlencode(state: LuaState) -> c_int {
    laux::lua_checktype(state, 1, ffi::LUA_TTABLE);

    let mut result = String::with_capacity(64);
    for (key, value) in LuaTable::from_stack(state, 1).iter() {
        if !result.is_empty() {
            result.push('&');
        }
        result.push_str(
            form_urlencoded::byte_serialize(key.to_vec().as_ref())
                .collect::<String>()
                .as_str(),
        );
        result.push('=');
        result.push_str(
            form_urlencoded::byte_serialize(value.to_vec().as_ref())
                .collect::<String>()
                .as_str(),
        );
    }
    laux::lua_push(state, result);
    1
}

extern "C-unwind" fn lua_http_form_urldecode(state: LuaState) -> c_int {
    let query_string = laux::lua_get::<&str>(state, 1);

    let decoded: Vec<(String, String)> = form_urlencoded::parse(query_string.as_bytes())
        .into_owned()
        .collect();

    let table = LuaTable::new(state, 0, decoded.len());

    for (key, value) in decoded {
        table.rawset(key, value);
    }
    1
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C-unwind" fn luaopen_rust_httpc(state: LuaState) -> c_int {
    let l = [
        lreg!("request", lua_http_request),
        lreg!("decode", decode),
        lreg!("form_urlencode", lua_http_form_urlencode),
        lreg!("form_urldecode", lua_http_form_urldecode),
        lreg_null!(),
    ];

    luaL_newlib!(state, l);

    1
}
