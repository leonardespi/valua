-- INFERRED: transformation rule from PRD §5.1; bit.rshift is LOGICAL (unsigned): bit.rshift(-1,1)=2147483647 VERIFIED via probe 1.3
-- Evidence: EVIDENCE.md §"Probe 1.3" — >> maps to bit.rshift (logical); use bit.arshift for arithmetic right shift

local a = bit.rshift(256, 2)
local b = bit.rshift(0xFF00, 8)
return a, b
