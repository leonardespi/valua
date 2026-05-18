-- Tests rewriting of binary `&` to bit.band() call.
-- Includes chained case to verify left-associativity is preserved.

local a = 0xFF & 0x0F
local b = 0xAA & 0x55 & 0x0F
return a, b
