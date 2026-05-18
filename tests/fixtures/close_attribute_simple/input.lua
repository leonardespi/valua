-- Tests rewriting of <close> attribute (happy path) to pcall-based cleanup.
-- The body succeeds; close handler must be called after body completes.

local f <close> = io.open("file.txt", "r")
local data = f:read("*a")
return data
