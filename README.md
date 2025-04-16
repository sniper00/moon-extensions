# Moon Extensions

This library provides Lua extensions for [Moon](https://github.com/sniper00/moon), implemented in Rust and C/C++. By using Rust, we can use its ecosystem, including the `tokio` runtime.

# Usage

## Option 1: Use Precompiled Releases

You can directly use the precompiled releases. [Download](https://github.com/sniper00/moon-extensions/releases/tag/prebuilt).

## Option 2: Manual Compilation

To compile the project, follow these steps:

- `git clone --recursive https://github.com/sniper00/moon-extensions.git`
- [Install Premake5](https://premake.github.io/download).
- Run `premake5 build` `premake5 publish`.

After compiling, the `clib` and `lualib` directories will be automatically copied to the moon directory.

# Libraries

## Rust

### 1. Excel Reader

```lua
local excel = require "rust.excel"

local res = excel.read("example.xlsx")

--[[
---format
{
    {
        "data":{
            {"A1","B1","C1"},
            {"A2","B2","C2"},
            {"A3","B3","C3"}
        },
        "sheet_name":"Sheet1"
    },
    {
        "data":{
            {"A1","B1","C1"},
            {"A2","B2","C2"},
            {"A3","B3","C3"}
        },
        "sheet_name":"Sheet2"
    }
}
]]

```

### 2. Https Client

```lua
    local httpc = require("ext.httpc")
    local moon = require("moon")
    moon.async(function()
        local response = httpc.get("https://bing.com")
        print(response.status_code==200)
    end)
```

### 3. Sqlx

```lua
local moon = require "moon"
local sqlx = require "ext.sqlx"

moon.loglevel("INFO")

moon.async(function()
    local info = {
        cc      = 300,
        gpsname = "gps1",
        track   = {
            segments = {
                [1] = {
                    HR        = 73,
                    location  = {
                        [1] = 47.763,
                        [2] = 13.4034,
                    },
                    starttime = "2018-10-14 10:05:14",
                },
                [2] = {
                    HR        = 130,
                    location  = {
                        [1] = 47.706,
                        [2] = 13.2635,
                    },
                    starttime = "2018-10-14 10:39:21",
                },
            },
        },
    }

    local sql = string.format([[
        drop table if exists userdata;
    ]])

    local sql2 = [[
        --create userdata table
        create table userdata (
            uid	bigint,
            key		text,
            value   text,
            CONSTRAINT pk_userdata PRIMARY KEY (uid, key)
           );
    ]]


    local db = sqlx.connect("postgres://bruce:123456@localhost/postgres", "test")
    print(db)
    if db.kind then
        print("connect failed", db.message)
        return
    end

    print_r(db:transaction({
        {sql},
        {sql2},
    }))

    local result = db:query(
        "INSERT INTO userdata (uid, key, value) values($1, $2, $3) on conflict (uid, key) do update set value = excluded.value;",
        235, "info2", info)
    print_r(result)

    local st = moon.clock()
    local trans = {}
    for i = 1, 10000 do
        trans[#trans+1] = {"INSERT INTO userdata (uid, key, value) values($1, $2, $3) on conflict (uid, key) do update set value = excluded.value;", 235, "info2", info}
    end
    print_r(db:transaction(trans))
    print("trans cost", moon.clock() - st)

    local st = moon.clock()
    for i = 10001, 20000 do
        local res = db:query(
            "INSERT INTO userdata (uid, key, value) values($1, $2, $3) on conflict (uid, key) do update set value = excluded.value;",
            235, "info2", info)

        if res.kind then
            print("error", res.message)
            break
        end
    end
    print("cost", moon.clock() - st)

    ---sqlite
    local sqlitedb = sqlx.connect("sqlite://memory:", "test2")

    print_r(sqlitedb:query("CREATE TABLE test (id INTEGER PRIMARY KEY, content TEXT);"))
    print_r(sqlitedb:query("INSERT INTO test (content) VALUES ('Hello, World!');"))
    print_r(sqlitedb:query("SELECT * FROM test;"))

    print_r(sqlx.stats()) -- Query sqlx left task count
end)


```

### 4. MongoDB

```lua
local moon = require "moon"

local mongodb = require "ext.mongodb"

moon.async(function()
    local db = mongodb.connect("mongodb://127.0.0.1:27017/?serverSelectionTimeoutMS=2000", "gamedb1")
    if db.kind then
        print("connect failed", db.message)
        return
    end

    local coll = db:collection("mydatabase", "mycollection")

    local res = coll:insert_one({
        cc      = 300,
        gpsname = "gps1",
        track   = {
            segments = {
                [1] = {
                    HR        = 73,
                    location  = {
                        [1] = 47.763,
                        [2] = 13.4034,
                    },
                    starttime = "2018-10-14 10:05:14",
                },
                [2] = {
                    HR        = 130,
                    location  = {
                        [1] = 47.706,
                        [2] = 13.2635,
                    },
                    starttime = "2018-10-14 10:39:21",
                },
            },
        },
    })

    print_r(res)

    res = coll:update_one({cc = 300}, {
        ["$set"] = {
            ["track.segments.1.HR"] = 100,
        }
    })

    print_r(res)

    res = coll:find({cc = 300}, {limit = 10})

    print_r(res)

    res = coll:find_one({cc = 300})
    print_r(res)

    print_r(coll:delete_one({cc = 300}))

    print_r(coll:delete_many({cc = 300}))

    res = coll:find_one({cc = 300})
    print_r(res)

    res = coll:insert_one({
        cc      = 300,
        gpsname = "gps1",
        track   = {
            segments = {
                [1] = {
                    HR        = 73,
                    location  = {
                        [1] = 47.763,
                        [2] = 13.4034,
                    },
                    starttime = "2018-10-14 10:05:14",
                },
                [2] = {
                    HR        = 130,
                    location  = {
                        [1] = 47.706,
                        [2] = 13.2635,
                    },
                    starttime = "2018-10-14 10:39:21",
                },
            },
        },
    })

    print_r(res)

    local bt = moon.clock()
    for i=1,10000 do
        coll:insert_one({
            cc      = 300,
            gpsname = "gps1",
            track   = {
                segments = {
                    [1] = {
                        HR        = 73,
                        location  = {
                            [1] = 47.763,
                            [2] = 13.4034,
                        },
                        starttime = "2018-10-14 10:05:14",
                    },
                    [2] = {
                        HR        = 130,
                        location  = {
                            [1] = 47.706,
                            [2] = 13.2635,
                        },
                        starttime = "2018-10-14 10:39:21",
                    },
                },
            },
        })
    end
    print("insert 10000 use time", moon.clock() - bt)

end)
```

## C/Cpp

### 1. lpeg

### 2. [math3d](https://github.com/cloudwu/math3d)

### 3. [sproto](https://github.com/cloudwu/sproto)
