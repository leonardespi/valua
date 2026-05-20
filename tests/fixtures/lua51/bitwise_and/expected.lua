-- VERIFIED: output captured from Compiler::compile with CompileOptions::lua51() on 2026-05-20
-- Evidence: O2 close — first Lua 5.1 end-to-end integration fixture

local _bU = 4294967296
local _bS = 2147483648
local function _bu(n)
    n = math.floor(n) % _bU
    if n < 0 then
        n = n + _bU
    end
    return n
end
local function _bs(n)
    n = _bu(n)
    if n >= _bS then
        n = n - _bU
    end
    return n
end
local bit = {}
function bit.band(a, b)
    a = _bu(a)
    b = _bu(b)
    local r = 0
    local m = 1
    while a > 0 or b > 0 do
        if a % 2 == 1 and b % 2 == 1 then
            r = r + m
        end
        a = math.floor(a / 2)
        b = math.floor(b / 2)
        m = m * 2
    end
    return _bs(r)
end
function bit.bor(a, b)
    a = _bu(a)
    b = _bu(b)
    local r = 0
    local m = 1
    while a > 0 or b > 0 do
        if a % 2 == 1 or b % 2 == 1 then
            r = r + m
        end
        a = math.floor(a / 2)
        b = math.floor(b / 2)
        m = m * 2
    end
    return _bs(r)
end
function bit.bxor(a, b)
    a = _bu(a)
    b = _bu(b)
    local r = 0
    local m = 1
    while a > 0 or b > 0 do
        if a % 2 ~= b % 2 then
            r = r + m
        end
        a = math.floor(a / 2)
        b = math.floor(b / 2)
        m = m * 2
    end
    return _bs(r)
end
function bit.bnot(a)
    return _bs(4294967295 - _bu(a))
end
function bit.lshift(a, n)
    n = n % 32
    a = _bu(a)
    for _ = 1, n do
        a = a * 2 % _bU
    end
    return _bs(a)
end
function bit.rshift(a, n)
    n = n % 32
    a = _bu(a)
    for _ = 1, n do
        a = math.floor(a / 2)
    end
    return _bs(a)
end
local a = bit.band(255, 15)
local b = bit.band(bit.band(170, 85), 15)
return a, b
