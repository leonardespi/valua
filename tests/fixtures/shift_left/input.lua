-- Tests rewriting of `<<` (left shift) to bit.lshift() call.

local a = 1 << 4
local b = 0x01 << 8
return a, b
