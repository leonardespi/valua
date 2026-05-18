-- Tests rewriting of `//` (floor division) to math.floor(a / b).
-- Negative dividend case confirms floor semantics (not truncation).

local a = 7 // 2
local b = -7 // 2
local c = 10 // 3
return a, b, c
