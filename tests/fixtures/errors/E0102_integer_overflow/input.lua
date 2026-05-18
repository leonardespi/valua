-- Triggers E0102: integer literal in arithmetic context where 64-bit overflow is observable.
-- Conservative syntactic detection: math.maxinteger used in arithmetic.

local max = 9223372036854775807
local overflow = max + 1
return overflow
