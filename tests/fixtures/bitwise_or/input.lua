-- Tests rewriting of binary `|` to bit.bor() call.
-- Includes chained case to verify left-associativity is preserved.

local a = 0x01 | 0x02
local b = 0x01 | 0x02 | 0x04
return a, b
