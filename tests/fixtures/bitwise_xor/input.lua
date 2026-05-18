-- Tests rewriting of binary `~` (XOR) to bit.bxor() call.
-- Binary ~ (XOR) must be distinguished from unary ~ (NOT).

local a = 0xFF ~ 0x0F
local b = 0xAA ~ 0x55 ~ 0x0F
return a, b
