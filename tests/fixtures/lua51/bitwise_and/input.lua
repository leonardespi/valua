-- Tests full Lua 5.1 compile path: polyfill injection + bit.band rewrite.
-- Same operators as bitwise_and fixture, target is lua51 (not luajit).

local a = 0xFF & 0x0F
local b = 0xAA & 0x55 & 0x0F
return a, b
