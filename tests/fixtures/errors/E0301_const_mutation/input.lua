-- Triggers E0301: reassignment of a <const>-attributed variable.
-- ConstMutation lint detects the write at line 4.

local x <const> = 42
x = 99
return x
