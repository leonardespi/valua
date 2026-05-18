-- Tests rewriting of `>>` (right shift) to bit.rshift() call.
-- LuaJIT bit.rshift is logical (unsigned); differs from arithmetic rshift.

local a = 256 >> 2
local b = 0xFF00 >> 8
return a, b
