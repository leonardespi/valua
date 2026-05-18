-- VERIFIED: pcall-based close error path executed in LuaJIT 2.1.1703358377 in this session; exact sequence matched probe 1.4 test_error
-- Evidence: EVIDENCE.md §"Probe 1.4 — <close> emulation" — error(result,0) re-raise confirmed; __valua_close called before re-raise

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
local f = io.open("log.txt", "w")
local __valua_ok, __valua_result = pcall(function()
  f:write("start")
  error("intentional error")
end)
__valua_close(f)
if not __valua_ok then error(__valua_result, 0) end
