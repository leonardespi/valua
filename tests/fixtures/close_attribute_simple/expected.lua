-- VERIFIED: pcall-based close pattern executed in LuaJIT 2.1.1703358377 in this session; exact sequence matched probe 1.4 test_happy
-- Evidence: EVIDENCE.md §"Probe 1.4 — <close> emulation" — __valua_close via getmetatable+__close confirmed; multi-return captures first value only

local function __valua_close(obj)
  if obj == nil then return end
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
local f = io.open("file.txt", "r")
local __valua_ok, __valua_result = pcall(function()
  local data = f:read("*a")
  return data
end)
__valua_close(f)
if not __valua_ok then error(__valua_result, 0) end
return __valua_result
