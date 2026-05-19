-- INFERRED: transformation rule from PRD §5.1; left-associativity nesting confirmed via probe 1.5 (VERIFIED); bit.band values confirmed via probe 1.3
-- Evidence: EVIDENCE.md §"Probe 1.3", §"Probe 1.5"

local a = bit.band(255, 15)
local b = bit.band(bit.band(170, 85), 15)
return a, b
