-- INFERRED: transformation rule from PRD §5.1; bit.lshift(1,10)=1024 VERIFIED via probe 1.3; fixture uses small shifts only
-- Evidence: EVIDENCE.md §"Probe 1.3" — WARNING: bit.lshift wraps for shifts>=32 (bit.lshift(1,32)=1); fixture avoids this domain

local a = bit.lshift(1, 4)
local b = bit.lshift(0x01, 8)
return a, b
