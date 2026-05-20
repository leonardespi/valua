-- VERIFIED: output captured from Compiler::compile with CompileOptions::luajit() on 2026-05-20
-- Evidence: O6 regression fixture — __valua_close injected twice (outer scope + inner pcall); functionally correct via shadowing

local function __valua_close(obj)
    if obj == nil then
        return
    end
    local mt = getmetatable(obj)
    if mt and mt.__close then
        mt.__close(obj)
        return
    end
    if type(obj) == "table" and type(obj.close) == "function" then
        obj:close()
        return
    end
end
local f = io.open("a.txt", "r")
local __valua_ok, __valua_result = pcall(function()
    local function __valua_close(obj)
        if obj == nil then
            return
        end
        local mt = getmetatable(obj)
        if mt and mt.__close then
            mt.__close(obj)
            return
        end
        if type(obj) == "table" and type(obj.close) == "function" then
            obj:close()
            return
        end
    end
    local g = io.open("b.txt", "r")
    local __valua_ok, __valua_result = pcall(function()
        return f, g
    end)
    __valua_close(g)
    if not __valua_ok then
        error(__valua_result, 0)
    end
    return __valua_result
end)
__valua_close(f)
if not __valua_ok then
    error(__valua_result, 0)
end
return __valua_result
