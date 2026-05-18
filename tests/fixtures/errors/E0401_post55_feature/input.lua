-- Triggers E0401: post-Lua-5.5 syntax recognized but not yet supported.
-- Placeholder using hypothetical attribute from a future Lua version (5.6+).
-- valua 1.0 targets Lua 5.5; this fixture marks the next unsupported boundary.
-- Update when a confirmed post-5.5 syntax construct is identified.

local x <future_attr> = 42
return x
