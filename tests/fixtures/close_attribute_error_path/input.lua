-- Tests rewriting of <close> attribute (error path) to pcall-based cleanup.
-- The body throws; close handler must be called and the error re-raised.

local f <close> = io.open("log.txt", "w")
f:write("start")
error("intentional error")
