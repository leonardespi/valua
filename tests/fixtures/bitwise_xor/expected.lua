-- INFERRED: transformation rule from PRD §5.1; left-associativity nesting confirmed via probe 1.5 (VERIFIED); bit.bxor(0xAA,0x55)=255 confirmed via probe 1.3
-- Evidence: EVIDENCE.md §"Probe 1.3", §"Probe 1.5"

local a = bit.bxor(0xFF, 0x0F)
local b = bit.bxor(bit.bxor(0xAA, 0x55), 0x0F)
return a, b
