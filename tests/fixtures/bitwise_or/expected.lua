-- INFERRED: transformation rule from PRD §5.1; left-associativity nesting confirmed via probe 1.5 (VERIFIED); bit.bor values confirmed via probe 1.3
-- Evidence: EVIDENCE.md §"Probe 1.3", §"Probe 1.5"

local a = bit.bor(1, 2)
local b = bit.bor(bit.bor(1, 2), 4)
return a, b
