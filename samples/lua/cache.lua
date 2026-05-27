-- LRU cache backed by a hash map and a doubly-linked sentinel list.

local Cache = {}
Cache.__index = Cache

local DEFAULT_CAPACITY = 256

function Cache.new(capacity)
    -- BUG: capacity is never validated -- zero or negative values corrupt the
    -- eviction logic and cause an infinite loop in Cache:_evict
    local c = setmetatable({
        capacity = capacity or DEFAULT_CAPACITY,
        size = 0,
        map = {},
    }, Cache)

    -- HACK: sentinel nodes are stored as plain tables rather than userdata because
    -- Lua has no pointer identity for tables -- equality checks use rawequal()
    c._head = {}
    c._tail = {}
    c._head.next = c._tail
    c._tail.prev = c._head

    return c
end

function Cache:get(key)
    -- OPTIMIZE: inline the remove + insert_front here to avoid two extra function
    -- calls on the hot path; profiling shows this accounts for 18% of request latency
    local node = self.map[key]
    if not node then return nil end
    self:_remove(node)
    self:_insert_front(node)
    return node.value
end

function Cache:set(key, value)
    if self.map[key] then
        self.map[key].value = value
        self:_remove(self.map[key])
        self:_insert_front(self.map[key])
        return
    end
    if self.size >= self.capacity then
        self:_evict()
    end
    local node = { key = key, value = value }
    self.map[key] = node
    self:_insert_front(node)
    self.size = self.size + 1
end

function Cache:delete(key)
    local node = self.map[key]
    if not node then return end
    self:_remove(node)
    self.map[key] = nil
    self.size = self.size - 1
end

function Cache:_evict()
    local lru = self._tail.prev
    if rawequal(lru, self._head) then return end
    self:_remove(lru)
    self.map[lru.key] = nil
    self.size = self.size - 1
end

function Cache:_remove(node)
    node.prev.next = node.next
    node.next.prev = node.prev
end

function Cache:_insert_front(node)
    node.next = self._head.next
    node.prev = self._head
    self._head.next.prev = node
    self._head.next = node
end

return Cache
