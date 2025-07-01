# Moon Extensions

This library provides Lua extensions for [Moon](https://github.com/sniper00/moon), implemented in Rust and C/C++. By using Rust, we can use its ecosystem, including the `tokio` runtime.

# Usage

## Option 1: Use Precompiled Releases

You can directly use the precompiled releases. [Download](https://github.com/sniper00/moon-extensions/releases/tag/prebuilt).

## Option 2: Manual Compilation

To compile the project, follow these steps:

- `git clone --recursive https://github.com/sniper00/moon-extensions.git`
- [Install Premake5](https://premake.github.io/download).
- [Install Rust](https://www.rust-lang.org/)
- Make sure your compiler(vs2022 17.5+, gcc 9.3+, clang 9.0+) support C++17
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

### 5. Crypto

```lua
-- AES Encryption/Decryption Test Script
local crypto = require("rust.crypto")

print("=== AES Encryption/Decryption Test ===")

-- Test 1: Basic encryption/decryption
print("\n1. Basic encryption/decryption test:")

-- Generate random key
local key = crypto.generate_key() -- Default 32 bytes for AES-256
print("Generated key length:", #key, "bytes")

-- Data to encrypt
local data = "Hello, AES-GCM encryption!"
print("Original data:", data)

-- Encrypt data
local encrypted_data, nonce = crypto.aes_encrypt(data, key)
print("Encrypted data length:", #encrypted_data, "bytes")
print("Nonce length:", #nonce, "bytes")

-- Decrypt data
local decrypted_data = crypto.aes_decrypt(encrypted_data, key, nonce)
print("Decrypted data:", decrypted_data)

-- Verify result
if data == decrypted_data then
    print("✓ Encryption/decryption test passed")
else
    print("✗ Encryption/decryption test failed")
end

-- Test 2: Using custom nonce
print("\n2. Custom nonce test:")

local custom_nonce = crypto.generate_nonce()
print("Custom nonce length:", #custom_nonce, "bytes")

local data2 = "Testing with custom nonce"
local encrypted_data2, returned_nonce = crypto.aes_encrypt(data2, key, custom_nonce)
local decrypted_data2 = crypto.aes_decrypt(encrypted_data2, key, custom_nonce)

print("Original data:", data2)
print("Decrypted data:", decrypted_data2)

if data2 == decrypted_data2 then
    print("✓ Custom nonce test passed")
else
    print("✗ Custom nonce test failed")
end

-- Test 3: Generate keys of different lengths
print("\n3. Different key length test:")

local key16 = crypto.generate_key(16) -- 16 bytes for AES-128
local key24 = crypto.generate_key(24) -- 24 bytes for AES-192
local key32 = crypto.generate_key(32) -- 32 bytes for AES-256

print("16-byte key length:", #key16)
print("24-byte key length:", #key24)
print("32-byte key length:", #key32)

-- Test 4: AES-128 encryption/decryption
print("\n4. AES-128 encryption/decryption test:")

local aes128_key = crypto.generate_key(16) -- 16 bytes for AES-128
local aes128_data = "Testing AES-128 GCM encryption!"
print("AES-128 key length:", #aes128_key, "bytes")
print("Original data:", aes128_data)

-- Encrypt with AES-128
local aes128_encrypted, aes128_nonce = crypto.aes_encrypt(aes128_data, aes128_key)
print("AES-128 encrypted data length:", #aes128_encrypted, "bytes")
print("AES-128 nonce length:", #aes128_nonce, "bytes")

-- Decrypt with AES-128
local aes128_decrypted = crypto.aes_decrypt(aes128_encrypted, aes128_key, aes128_nonce)
print("AES-128 decrypted data:", aes128_decrypted)

-- Verify AES-128 result
if aes128_data == aes128_decrypted then
    print("✓ AES-128 encryption/decryption test passed")
else
    print("✗ AES-128 encryption/decryption test failed")
end

-- Test 5: Binary data encryption/decryption
print("\n5. Binary data test:")

local binary_data = string.rep("\x00\x01\x02\x03\x04\x05\x06\x07", 10)
print("Binary data length:", #binary_data, "bytes")

local encrypted_binary, nonce_binary = crypto.aes_encrypt(binary_data, key)
local decrypted_binary = crypto.aes_decrypt(encrypted_binary, key, nonce_binary)

if binary_data == decrypted_binary then
    print("✓ Binary data test passed")
else
    print("✗ Binary data test failed")
end

-- Test 6: AES-128 vs AES-256 comparison
print("\n6. AES-128 vs AES-256 comparison test:")

local test_data = "This is a test message for comparing AES-128 and AES-256 performance and functionality."
local aes128_key_comp = crypto.generate_key(16) -- AES-128 key
local aes256_key_comp = crypto.generate_key(32) -- AES-256 key

print("Test data:", test_data)
print("Test data length:", #test_data, "bytes")

-- AES-128 encryption
local aes128_enc, aes128_nonce_comp = crypto.aes_encrypt(test_data, aes128_key_comp)
local aes128_dec = crypto.aes_decrypt(aes128_enc, aes128_key_comp, aes128_nonce_comp)

-- AES-256 encryption  
local aes256_enc, aes256_nonce_comp = crypto.aes_encrypt(test_data, aes256_key_comp)
local aes256_dec = crypto.aes_decrypt(aes256_enc, aes256_key_comp, aes256_nonce_comp)

print("AES-128 encrypted length:", #aes128_enc, "bytes")
print("AES-256 encrypted length:", #aes256_enc, "bytes")

-- Verify both results
local aes128_ok = (test_data == aes128_dec)
local aes256_ok = (test_data == aes256_dec)

if aes128_ok and aes256_ok then
    print("✓ Both AES-128 and AES-256 comparison test passed")
elseif aes128_ok then
    print("✓ AES-128 passed, ✗ AES-256 failed")
elseif aes256_ok then
    print("✗ AES-128 failed, ✓ AES-256 passed")
else
    print("✗ Both AES-128 and AES-256 comparison test failed")
end

print("\n=== Test Complete ===")

```

## C/Cpp

### 1. [lpeg](https://github.com/roberto-ieru/LPeg)

### 2. [math3d](https://github.com/cloudwu/math3d)

### 3. [sproto](https://github.com/cloudwu/sproto)
