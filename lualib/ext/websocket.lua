---@diagnostic disable: inject-field
local moon = require "moon"
local c = require "rust.websocket"

local protocol_type = 25

moon.register_protocol {
    name = "websocket_rs",
    PTYPE = protocol_type,
    pack = function(...) return ... end,
    unpack = function(val)
        return c.decode(val)
    end
}

---@class Websocket
---@field obj any
---@field response? HttpResponse
---@field id integer
local M = {}

---@async
---@nodiscard
---@param url string Database url e. "wss://example.com/socket"
---@param timeout? integer Connect timeout. Default 5000ms
---@return Websocket
function M.connect(url, timeout)
    local response = moon.wait(c.connect({
        protocol_type = protocol_type,
        owner = moon.id,
        session = moon.next_sequence(),
        url = url,
        connect_timeout = timeout or 5000
    }))

    if not response then
        error(string.format("connect failed: %s", response))
    end

    local o = {
        obj = c.find_connection(response.fd),
        id = response.fd,
        response = response
    }
    return setmetatable(o, { __index = M })
end

---@nodiscard
---@param id integer Connection id
---@return Websocket
function M.find_connection(id)
    local o = {
        obj = c.find_connection(id),
        id = id,
    }
    return setmetatable(o, { __index = M })
end

---@nodiscard
---@async
---@param timeout? integer Timeout in milliseconds. Default 5000ms
function M:read(timeout)
    return moon.wait(self.obj:read(moon.id, moon.next_sequence(), timeout or 5000))
end

---@param data string
function M:write(data)
    return self.obj:write(data)
end

---@param data string
function M:write_text(data)
    return self.obj:write(data, "t")
end

---@param data string
function M:write_ping(data)
    return self.obj:write(data, "p")
end

function M:close()
    self.obj:close()
end

return M
