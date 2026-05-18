-- Triggers E0101: call to math.type() which is not representable in LuaJIT.
-- math.type() observes the integer/float distinction absent in LuaJIT.

local x = 1
local kind = math.type(x)
return kind
