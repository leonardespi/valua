-- Tests erasure of <const> attribute: emitted as plain `local`.
-- Static analysis (ConstMutation lint) validates no reassignment occurs.

local x <const> = 42
local message <const> = "hello"
return x, message
