//! Demonstrates the public valua-core API for transpiling Lua 5.4 → 5.1.
//!
//! Run with: `cargo run --example basic_transpile`

use valua_core::{CompileOptions, Compiler};

fn main() {
    let source = r"
-- Lua 5.4 source: bitwise ops + <const> + integer division
local x <const> = 10
local y <const> = 3
local bits = x & y        -- bitwise AND
local idiv = x // y       -- integer division
print(bits, idiv)
";

    // ── Example 1: default options (Lua 5.1) ─────────────────────────────────
    let opts = CompileOptions::default();
    println!("=== Target: Lua 5.1 ===");
    match Compiler::compile(source, opts) {
        Ok(output) => println!("{output}"),
        // Expected at this stage — pipeline is not yet implemented.
        Err(e) => eprintln!("compile error (expected at skeleton stage): {e}"),
    }

    // ── Example 2: LuaJIT target ──────────────────────────────────────────────
    let opts_jit = CompileOptions::luajit();
    println!("=== Target: LuaJIT ===");
    match Compiler::compile(source, opts_jit) {
        Ok(output) => println!("{output}"),
        Err(e) => eprintln!("compile error (expected at skeleton stage): {e}"),
    }

    // ── Example 3: parse-only (syntax check) ─────────────────────────────────
    println!("=== Syntax check only ===");
    match Compiler::parse_only(source) {
        Ok(block) => println!("Parsed {} top-level statements.", block.stmts.len()),
        Err(e) => eprintln!("parse error (expected at skeleton stage): {e}"),
    }
}
