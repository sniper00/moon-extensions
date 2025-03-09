---@diagnostic disable: inject-field
local moon = require "moon"
local c = require "rust.sqlx"

local protocol_type = 23

moon.register_protocol {
    name = "database",
    PTYPE = protocol_type,
    pack = function(...) return ... end,
    unpack = function(val)
        return c.decode(val)
    end
}

---@class SqlX
local M = {}

---@nodiscard
---@param database_url string Database url e. "postgres://postgres:123456@localhost/postgres"
---@param name string Connection name for find by other services
---@param timeout? integer Connect timeout. Default 5000ms
---@return SqlX
function M.connect(database_url, name, timeout)
    local res = moon.wait(c.connect(protocol_type, moon.id, moon.next_sequence(), database_url, name, timeout))
    if res.kind then
        error(string.format("connect database failed: %s", res.message))
    end
    return M.find_connection(name)
end

---@nodiscard
---@param name string Connection name
---@return SqlX
function M.find_connection(name)
    local o = {
        obj = c.find_connection(name)
    }
    return setmetatable(o, { __index = M })
end

function M.stats()
    return c.stats()
end

function M:close()
    self.obj:close()
end

---@param sql string
---@vararg any
function M:execute(sql, ...)
    local res = self.obj:query(moon.id, 0, sql, ...)
    if type(res) == "table" then
        moon.error(print_r(res, true))
    end
end

---@nodiscard
---@param sql string
---@vararg any
---@return table
function M:query(sql, ...)
    local session = self.obj:query(moon.id, moon.next_sequence(), sql, ...)
    if type(session) == "table" then
        return session
    end
    return moon.wait(session)
end

---@async
---@nodiscard
---@param querys table
---@return table
function M:transaction(querys)
    local trans = c.make_transaction()
    for _, v in ipairs(querys) do
        trans:push(table.unpack(v))
    end
    local session = self.obj:transaction(moon.id, moon.next_sequence(), trans)
    if type(session) == "table" then
        return session
    end
    return moon.wait(session)
end

---@param querys table
function M:execute_transaction(querys)
    local trans = c.make_transaction()
    for _, v in ipairs(querys) do
        trans:push(table.unpack(v))
    end
    local res = self.obj:transaction(moon.id, 0, trans)
    if type(res) == "table" then
        moon.error(print_r(res, true))
    end
end

return M
