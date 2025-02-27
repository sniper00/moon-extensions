use crate::lua_json::{encode_one, JsonOptions};
use crate::{moon_log, moon_send, LOG_LEVEL_ERROR, LOG_LEVEL_INFO};
use dashmap::DashMap;
use lazy_static::lazy_static;
use lib_core::context::CONTEXT;
use lib_lua::laux::{lua_into_userdata, LuaTable};
use lib_lua::luaL_newlib;
use lib_lua::{self, cstr, ffi, ffi::luaL_Reg, laux, lreg, lreg_null, push_lua_table};
use sqlx::migrate::MigrateDatabase;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::sqlite::SqliteRow;
use sqlx::ColumnIndex;
use sqlx::{
    Column, Database, MySql, MySqlPool, PgPool, Postgres, Row, Sqlite, SqlitePool, TypeInfo,
};
use std::ffi::c_int;
use std::sync::atomic::AtomicI64;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

lazy_static! {
    static ref DATABASE_CONNECTIONSS: DashMap<String, DatabaseConnection> = DashMap::new();
}

enum DatabasePool {
    MySql(MySqlPool),
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

impl DatabasePool {
    async fn connect(database_url: &str, timeout_duration: Duration) -> Result<Self, sqlx::Error> {
        async fn connect_with_timeout<F, T>(
            timeout_duration: Duration,
            connect_future: F,
        ) -> Result<T, sqlx::Error>
        where
            F: std::future::Future<Output = Result<T, sqlx::Error>>,
        {
            timeout(timeout_duration, connect_future)
                .await
                .map_err(|err| {
                    sqlx::Error::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Connection error: {}", err),
                    ))
                })?
        }

        if database_url.starts_with("mysql://") {
            let pool =
                connect_with_timeout(timeout_duration, MySqlPool::connect(database_url)).await?;
            Ok(DatabasePool::MySql(pool))
        } else if database_url.starts_with("postgres://") {
            let pool = connect_with_timeout(
                timeout_duration,
                PgPoolOptions::new()
                    .max_connections(1)
                    .acquire_timeout(Duration::from_secs(2))
                    .connect(database_url),
            )
            .await?;
            Ok(DatabasePool::Postgres(pool))
        } else if database_url.starts_with("sqlite://") {
            if !Sqlite::database_exists(database_url).await? {
                Sqlite::create_database(database_url).await?;
            }
            let pool =
                connect_with_timeout(timeout_duration, SqlitePool::connect(database_url)).await?;
            Ok(DatabasePool::Sqlite(pool))
        } else {
            Err(sqlx::Error::Configuration(
                "Unsupported database type".into(),
            ))
        }
    }

    fn make_query<'a, DB: sqlx::Database>(
        sql: &'a str,
        binds: &'a [QueryParams],
    ) -> Result<sqlx::query::Query<'a, DB, <DB as sqlx::Database>::Arguments<'a>>, sqlx::Error>
    where
        bool: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
        i64: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
        f64: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
        &'a str: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
        serde_json::Value: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
        &'a Vec<u8>: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    {
        let mut query = sqlx::query(sql);
        for bind in binds {
            query = match bind {
                QueryParams::Bool(value) => query.bind(*value),
                QueryParams::Int(value) => query.bind(*value),
                QueryParams::Float(value) => query.bind(*value),
                QueryParams::Text(value) => query.bind(value.as_str()),
                QueryParams::Json(value) => query.bind(value),
                QueryParams::Bytes(value) => query.bind(value),
            };
        }
        Ok(query)
    }

    async fn query<'a>(&self, request: &DatabaseQuery) -> Result<DatabaseResult, sqlx::Error> {
        match self {
            DatabasePool::MySql(pool) => {
                let query = Self::make_query(&request.sql, &request.binds)?;
                let rows = query.fetch_all(pool).await?;
                Ok(DatabaseResult::MysqlRows(rows))
            }
            DatabasePool::Postgres(pool) => {
                let query = Self::make_query(&request.sql, &request.binds)?;
                let rows = query.fetch_all(pool).await?;
                Ok(DatabaseResult::PgRows(rows))
            }
            DatabasePool::Sqlite(pool) => {
                let query = Self::make_query(&request.sql, &request.binds)?;
                let rows = query.fetch_all(pool).await?;
                Ok(DatabaseResult::SqliteRows(rows))
            }
        }
    }
}

enum DatabaseOp {
    Query(i64, DatabaseQuery), // session, QueryBuilder
    Close(),
}

#[derive(Clone)]
struct DatabaseConnection {
    tx: mpsc::Sender<DatabaseOp>,
    counter: Arc<AtomicI64>,
}

enum DatabaseResult {
    Connect,
    PgRows(Vec<PgRow>),
    MysqlRows(Vec<MySqlRow>),
    SqliteRows(Vec<SqliteRow>),
    Error(sqlx::Error),
    Timeout(String),
}

#[derive(Debug, Clone)]
enum QueryParams {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Json(serde_json::Value),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
struct DatabaseQuery {
    sql: String,
    binds: Vec<QueryParams>,
}

async fn database_handler(
    protocol_type: u8,
    owner: u32,
    pool: &DatabasePool,
    mut rx: mpsc::Receiver<DatabaseOp>,
    database_url: &str,
    counter: Arc<AtomicI64>,
) {
    while let Some(op) = rx.recv().await {
        let mut failed_times = 0;
        match &op {
            DatabaseOp::Query(session, query_op) => loop {
                match pool.query(query_op).await {
                    Ok(rows) => {
                        moon_send(protocol_type, owner, *session, rows);
                        if failed_times > 0 {
                            moon_log(
                                owner,
                                LOG_LEVEL_INFO,
                                format!(
                                    "Database '{}' recover from error. Retry success.",
                                    database_url
                                ),
                            );
                        }
                        counter.fetch_sub(1, std::sync::atomic::Ordering::Release);
                        break;
                    }
                    Err(err) => {
                        let session = *session;
                        if session != 0 {
                            moon_send(protocol_type, owner, session, DatabaseResult::Error(err));
                            counter.fetch_sub(1, std::sync::atomic::Ordering::Release);
                            break;
                        } else {
                            if failed_times > 0 {
                                moon_log(
                                    owner,
                                    LOG_LEVEL_ERROR,
                                    format!(
                                        "Database '{}' error: '{:?}'. Will retry.",
                                        database_url,
                                        err.to_string()
                                    ),
                                );
                            }
                            failed_times += 1;
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
            },
            DatabaseOp::Close() => {
                break;
            }
        }
    }
}

extern "C-unwind" fn connect(state: *mut ffi::lua_State) -> c_int {
    let protocol_type: u8 = laux::lua_get(state, 1);
    let owner = laux::lua_get(state, 2);
    let session: i64 = laux::lua_get(state, 3);

    let database_url: &str = laux::lua_get(state, 4);
    let name: &str = laux::lua_get(state, 5);
    let connect_timeout: u64 = laux::lua_opt(state, 6).unwrap_or(5000);

    CONTEXT.tokio_runtime.spawn(async move {
        match DatabasePool::connect(database_url, Duration::from_millis(connect_timeout)).await {
            Ok(pool) => {
                let (tx, rx) = mpsc::channel(100);
                let counter = Arc::new(AtomicI64::new(0));
                DATABASE_CONNECTIONSS.insert(
                    name.to_string(),
                    DatabaseConnection {
                        tx: tx.clone(),
                        counter: counter.clone(),
                    },
                );
                moon_send(protocol_type, owner, session, DatabaseResult::Connect);
                database_handler(protocol_type, owner, &pool, rx, database_url, counter).await;
            }
            Err(err) => {
                moon_send(
                    protocol_type,
                    owner,
                    session,
                    DatabaseResult::Timeout(err.to_string()),
                );
            }
        };
    });

    laux::lua_push(state, session);
    1
}

extern "C-unwind" fn query(state: *mut ffi::lua_State) -> c_int {
    let conn = laux::lua_touserdata::<DatabaseConnection>(state, 1)
        .expect("Invalid database connect pointer");

    let options = JsonOptions::default();

    let session: i64 = laux::lua_get(state, 2);

    let sql = laux::lua_get::<&str>(state, 3);
    let mut params = Vec::new();
    let top = laux::lua_top(state);
    for i in 4..=top {
        let ltype = laux::lua_type(state, i);
        match ltype {
            laux::LuaType::Boolean => {
                if laux::lua_opt::<bool>(state, i).unwrap_or_default() {
                    params.push(QueryParams::Bool(true));
                } else {
                    params.push(QueryParams::Bool(false));
                }
            }
            laux::LuaType::Number => {
                if laux::is_integer(state, i) {
                    params.push(QueryParams::Int(laux::lua_to::<i64>(state, i)));
                } else {
                    params.push(QueryParams::Float(laux::lua_to::<f64>(state, i)));
                }
            }
            laux::LuaType::String => {
                let s = laux::lua_get::<&str>(state, i);
                if s.starts_with('{') || s.starts_with('[') {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(s) {
                        params.push(QueryParams::Json(value));
                    } else {
                        params.push(QueryParams::Text(s.to_string()));
                    }
                } else {
                    params.push(QueryParams::Text(s.to_string()));
                }
            }
            laux::LuaType::Table => {
                let mut buffer = Vec::new();
                if let Err(err) = encode_one(state, &mut buffer, i, 0, false, &options) {
                    drop(buffer);
                    drop(params);
                    laux::lua_error(state, &err);
                }
                if buffer[0] == b'{' || buffer[0] == b'[' {
                    if let Ok(value) =
                        serde_json::from_slice::<serde_json::Value>(buffer.as_slice())
                    {
                        params.push(QueryParams::Json(value));
                    } else {
                        params.push(QueryParams::Bytes(buffer));
                    }
                } else {
                    params.push(QueryParams::Bytes(buffer));
                }
            }
            _ => {
                drop(params);
                laux::lua_error(
                    state,
                    format!(
                        "concat: unsupport value type :{}",
                        laux::type_name(state, ltype as i32)
                    )
                    .as_str(),
                );
            }
        }
    }

    match conn.tx.try_send(DatabaseOp::Query(
        session,
        DatabaseQuery {
            sql: sql.to_string(),
            binds: params,
        },
    )) {
        Ok(_) => {
            conn.counter
                .fetch_add(1, std::sync::atomic::Ordering::Release);
            laux::lua_push(state, session);
            1
        }
        Err(err) => {
            push_lua_table!(
                state,
                "kind" => "ERROR",
                "message" => err.to_string()
            );
            1
        }
    }
}

extern "C-unwind" fn close(state: *mut ffi::lua_State) -> c_int {
    let conn = laux::lua_touserdata::<DatabaseConnection>(state, 1)
        .expect("Invalid database connect pointer");

    match conn.tx.try_send(DatabaseOp::Close()) {
        Ok(_) => {
            laux::lua_push(state, true);
            1
        }
        Err(err) => {
            push_lua_table!(
                state,
                "kind" => "ERROR",
                "message" => err.to_string()
            );
            1
        }
    }
}

fn process_rows<'a, DB>(
    state: *mut ffi::lua_State,
    rows: &'a [<DB as Database>::Row],
) -> Result<i32, String>
where
    DB: sqlx::Database,
    usize: ColumnIndex<<DB as Database>::Row>,
    bool: sqlx::Decode<'a, DB>,
    i64: sqlx::Decode<'a, DB>,
    f64: sqlx::Decode<'a, DB>,
    &'a str: sqlx::Decode<'a, DB>,
    &'a [u8]: sqlx::Decode<'a, DB>,
{
    let table = LuaTable::new(state, rows.len(), 0);
    if rows.is_empty() {
        return Ok(1);
    }

    let mut column_info = Vec::new();
    if column_info.is_empty() {
        rows.iter()
            .next()
            .unwrap()
            .columns()
            .iter()
            .enumerate()
            .for_each(|(index, column)| {
                column_info.push((index, column.name(), column.type_info().name()));
            });
    }

    let mut i = 0;
    for row in rows.iter() {
        let row_table = LuaTable::new(state, 0, row.len());
        for (index, column_name, column_type_name) in column_info.iter() {
            match row.try_get_raw(*index) {
                Ok(value) => match *column_type_name {
                    "NULL" => {
                        row_table.set(*column_name, ffi::LUA_TNIL);
                    }
                    "BOOL" | "BOOLEAN" => {
                        row_table.set(
                            *column_name,
                            sqlx::decode::Decode::decode(value).unwrap_or(false),
                        );
                    }
                    "INT2" | "INT4" | "INT8" | "TINYINT" | "SMALLINT" | "INT" | "MEDIUMINT"
                    | "BIGINT" | "INTEGER" => {
                        row_table.set(
                            *column_name,
                            sqlx::decode::Decode::decode(value).unwrap_or(0),
                        );
                    }
                    "FLOAT4" | "FLOAT8" | "NUMERIC" | "FLOAT" | "DOUBLE" | "REAL" => {
                        row_table.set(
                            *column_name,
                            sqlx::decode::Decode::decode(value).unwrap_or(0.0),
                        );
                    }
                    "TEXT" => {
                        row_table.set(
                            *column_name,
                            sqlx::decode::Decode::decode(value).unwrap_or(""),
                        );
                    }
                    _ => {
                        let column_value: &[u8] =
                            sqlx::decode::Decode::decode(value).unwrap_or(b"");
                        row_table.set(*column_name, column_value);
                    }
                },
                Err(error) => {
                    laux::lua_push(state, false);
                    laux::lua_push(
                        state,
                        format!("{:?} decode error: {:?}", column_name, error),
                    );
                    return Ok(2);
                }
            }
        }
        i += 1;
        table.seti(i);
    }
    Ok(1)
}

extern "C-unwind" fn find_connection(state: *mut ffi::lua_State) -> c_int {
    let name = laux::lua_get::<&str>(state, 1);
    match DATABASE_CONNECTIONSS.get(name) {
        Some(pair) => {
            let l = [lreg!("query", query), lreg!("close", close), lreg_null!()];
            if laux::lua_newuserdata(
                state,
                pair.value().clone(),
                cstr!("database_metatable"),
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

extern "C-unwind" fn decode(state: *mut ffi::lua_State) -> c_int {
    laux::luaL_checkstack(state, 6, std::ptr::null());
    let result = lua_into_userdata::<DatabaseResult>(state, 1);

    match *result {
        DatabaseResult::PgRows(rows) => {
            return process_rows::<Postgres>(state, &rows)
                .map_err(|e| {
                    push_lua_table!(
                        state,
                        "kind" => "ERROR",
                        "message" => e
                    );
                })
                .unwrap_or(1);
        }
        DatabaseResult::MysqlRows(rows) => {
            return process_rows::<MySql>(state, &rows)
                .map_err(|e| {
                    push_lua_table!(
                        state,
                        "kind" => "ERROR",
                        "message" => e
                    );
                })
                .unwrap_or(1);
        }
        DatabaseResult::SqliteRows(rows) => {
            return process_rows::<Sqlite>(state, &rows)
                .map_err(|e| {
                    push_lua_table!(
                        state,
                        "kind" => "ERROR",
                        "message" => e
                    );
                })
                .unwrap_or(1);
        }

        DatabaseResult::Connect => {
            push_lua_table!(
                state,
                "message" => "success"
            );
            return 1;
        }
        DatabaseResult::Error(err) => match err.as_database_error() {
            Some(db_err) => {
                push_lua_table!(
                    state,
                    "kind" => "DB",
                    "message" => db_err.message()
                );
            }
            None => {
                push_lua_table!(
                    state,
                    "kind" => "ERROR",
                    "message" => err.to_string()
                );
            }
        },
        DatabaseResult::Timeout(err) => {
            push_lua_table!(
                state,
                "kind" => "TIMEOUT",
                "message" => err.to_string()
            );
        }
    }

    1
}

extern "C-unwind" fn stats(state: *mut ffi::lua_State) -> c_int {
    let table = LuaTable::new(state, 0, DATABASE_CONNECTIONSS.len());
    DATABASE_CONNECTIONSS.iter().for_each(|pair| {
        table.set(
            pair.key().as_str(),
            pair.value()
                .counter
                .load(std::sync::atomic::Ordering::Acquire),
        );
    });
    1
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C-unwind" fn luaopen_rust_sqlx(state: *mut ffi::lua_State) -> c_int {
    let l = [
        lreg!("connect", connect),
        lreg!("find_connection", find_connection),
        lreg!("decode", decode),
        lreg!("stats", stats),
        lreg_null!(),
    ];

    luaL_newlib!(state, l);

    1
}
