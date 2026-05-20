-- Tests two <close> declarations in the same scope.
-- Known behavior: __valua_close helper is injected twice (outer + inner pcall).
-- Functionally correct — LIFO semantics preserved via nesting; shadowing is benign.
-- See Phase 5 open item O6 for the known redundancy.

local f <close> = io.open("a.txt", "r")
local g <close> = io.open("b.txt", "r")
return f, g
