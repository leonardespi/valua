-- INFERRED: <const> attribute erased to plain local per PRD §5.1; no runtime execution needed to verify this syntactic erasure

local x = 42
local message = "hello"
return x, message
