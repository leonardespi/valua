-- INFERRED: transformation rule from PRD §5.1; bit.bnot(0)=-1 and bit.bnot(0xFF)=-256 VERIFIED via probe 1.3 (32-bit signed domain)
-- Evidence: EVIDENCE.md §"Probe 1.3" — bit.bnot operates in 32-bit signed domain; differs from Lua 5.5 64-bit ~x for values > 2^31

local a = bit.bnot(0)
local b = bit.bnot(255)
return a, b
