-- Minimal HTTP router for a Lua/OpenResty application.

local Router = {}
Router.__index = Router

-- NOTE: path matching is plain string equality only -- no patterns, captures, or wildcards
local SUPPORTED_METHODS = { "GET", "POST", "PUT", "DELETE" }

function Router.new()
    return setmetatable({ routes = {}, middleware = {} }, Router)
end

function Router:use(fn)
    -- TODO: enforce middleware ordering guarantees -- currently insertion order is
    -- the only contract and nothing prevents auth from being registered after logging
    table.insert(self.middleware, fn)
end

function Router:register(method, path, handler)
    local key = method:upper() .. ":" .. path
    self.routes[key] = handler
end

function Router:dispatch(method, path, req, res)
    for _, mw in ipairs(self.middleware) do
        local stop = mw(req, res)
        if stop == false then return end
    end

    local key = method:upper() .. ":" .. path
    local handler = self.routes[key]

    if not handler then
        res:status(404):json({ error = "not found" })
        return
    end

    -- FIXME: errors thrown inside handler propagate to nginx's error log but the
    -- client receives a 502 with no body -- wrap in pcall and return a 500 instead
    handler(req, res)
end

function Router:routes_list()
    local list = {}
    for key in pairs(self.routes) do
        table.insert(list, key)
    end
    table.sort(list)
    return list
end

return Router
