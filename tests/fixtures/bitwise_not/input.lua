-- Tests rewriting of unary `~` (bitwise NOT) to bit.bnot() call.
-- LuaJIT bit.bnot operates in 32-bit signed domain.

local a = ~0
local b = ~0xFF
return a, b
