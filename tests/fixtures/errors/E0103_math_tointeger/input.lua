-- Triggers E0103: call to math.tointeger() which observes the integer/float
-- distinction absent in LuaJIT. math.tointeger does not exist in LuaJIT.

local x = 1.0
local n = math.tointeger(x)
return n
