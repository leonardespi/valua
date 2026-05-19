-- INFERRED: transformation rule from PRD §5.1; math.floor(-7/2)=-4 and math.floor(7/2)=3 VERIFIED via probe 1.3
-- Evidence: EVIDENCE.md §"Probe 1.3"

local a = math.floor(7 / 2)
local b = math.floor(- 7 / 2)
local c = math.floor(10 / 3)
return a, b, c
