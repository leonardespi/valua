//! Differential testing: `BITWISE_FALLBACK` pure-Lua polyfill vs. Rust 32-bit signed oracle.
//!
//! Spawns a real `luajit` process, executes the polyfill, and compares its stdout
//! line-by-line against values computed from Rust's integer arithmetic.
//!
//! **Skip behaviour:** if `luajit` is absent from PATH the whole suite prints a diagnostic
//! to stderr and returns without failure — it never hard-fails on a missing binary.
//!
//! **Oracle correctness:** the Rust oracle replicates the exact arithmetic of the
//! `_bu`/`_bs` helper functions embedded in `BITWISE_FALLBACK`:
//!   - `_bu(n)` → `n.rem_euclid(2^32) as u32`  (unsigned 32-bit projection)
//!   - `_bs(n)` → `n as i32 as i64`            (signed 32-bit interpretation)
//!
//! These two steps bracket every bitwise operation, so the Rust oracle is a
//! faithful algebraic translation of the Lua source, not an independent design.

#![allow(
    clippy::unreadable_literal,          // large hex/decimal literals in test vectors
    clippy::cast_sign_loss,              // rem_euclid(positive) is always non-negative
    clippy::cast_possible_truncation,    // rem_euclid(2^32) always fits in u32 by construction
    clippy::cast_possible_wrap,          // u32→i32 intentional sign-extension in `bs`
    clippy::uninlined_format_args,       // format!("{}", x) kept where it aids readability
)]

use std::process::{Command, Stdio};

use valua_polyfills::BITWISE_FALLBACK;

// ---------------------------------------------------------------------------
// Oracle — mirrors the Lua _bu / _bs arithmetic exactly
// ---------------------------------------------------------------------------

/// Project `n` into the unsigned 32-bit domain.
///
/// Matches Lua: `math.floor(n) % 0x100000000` (with the sign-correction branch
/// that Lua 5.1's modulo makes unnecessary but keeps for clarity).
#[inline]
fn bu(n: i64) -> u32 {
    // rem_euclid is always non-negative when divisor is positive → safe as u32.
    n.rem_euclid(0x1_0000_0000_i64) as u32
}

/// Reinterpret a u32 as a signed 32-bit integer widened to i64.
///
/// Matches Lua: `if n >= 0x80000000 then n - 0x100000000 else n end`.
#[inline]
fn bs(n: u32) -> i64 {
    i64::from(n as i32)
}

fn oracle_band(a: i64, b: i64) -> i64 {
    bs(bu(a) & bu(b))
}

fn oracle_bor(a: i64, b: i64) -> i64 {
    bs(bu(a) | bu(b))
}

fn oracle_bxor(a: i64, b: i64) -> i64 {
    bs(bu(a) ^ bu(b))
}

/// `0xFFFFFFFF - _bu(a)` is bitwise NOT of the 32-bit unsigned projection.
fn oracle_bnot(a: i64) -> i64 {
    bs(!bu(a))
}

/// Logical left shift, shift amount reduced mod 32.
fn oracle_lshift(a: i64, n: i64) -> i64 {
    let shift = n.rem_euclid(32_i64) as u32;
    bs(bu(a).wrapping_shl(shift))
}

/// Logical (zero-fill) right shift, shift amount reduced mod 32.
fn oracle_rshift(a: i64, n: i64) -> i64 {
    let shift = n.rem_euclid(32_i64) as u32;
    bs(bu(a).wrapping_shr(shift))
}

// ---------------------------------------------------------------------------
// Test-case construction
// ---------------------------------------------------------------------------

struct Case {
    /// The exact Lua expression passed to `print(...)`.
    lua_call: String,
    /// The value the expression must evaluate to.
    expected: i64,
}

/// Build the canonical test vector.
///
/// Values are chosen to cover:
/// - zero and one (additive / multiplicative identities)
/// - `i32::MIN` / `i32::MAX` (boundary values inside the signed domain)
/// - `-1` / `0xFFFFFFFF` (all-ones patterns; `0xFFFF_FFFF` is outside i32 range, tests masking)
/// - Alternating-nibble and alternating-bit patterns (`0x0F0F_0F0F`, `0xAAAA_AAAA`)
/// - A value below `i32::MIN` to exercise negative-input masking
///
/// Shift amounts cover zero (identity), byte boundaries (8, 16, 24), and
/// the critical single-bit-below-maximum (31).
fn build_cases() -> Vec<Case> {
    let vals: &[i64] = &[
        0,
        1,
        -1,
        127,
        -128,
        2_147_483_647,  // i32::MAX (0x7FFF_FFFF)
        -2_147_483_648, // i32::MIN (0x8000_0000 as signed)
        4_294_967_295,  // 0xFFFF_FFFF — out of i32 range, tests _bu masking
        252_645_135,    // 0x0F0F_0F0F — alternating nibbles
        2_863_311_530,  // 0xAAAA_AAAA — > i32::MAX, tests _bs sign-extension
    ];
    let shifts: &[i64] = &[0, 1, 7, 8, 15, 16, 24, 31];

    let mut cases = Vec::new();

    for &a in vals {
        for &b in vals {
            cases.push(Case {
                lua_call: format!("bit.band({a},{b})"),
                expected: oracle_band(a, b),
            });
            cases.push(Case {
                lua_call: format!("bit.bor({a},{b})"),
                expected: oracle_bor(a, b),
            });
            cases.push(Case {
                lua_call: format!("bit.bxor({a},{b})"),
                expected: oracle_bxor(a, b),
            });
        }
        cases.push(Case {
            lua_call: format!("bit.bnot({a})"),
            expected: oracle_bnot(a),
        });
        for &n in shifts {
            cases.push(Case {
                lua_call: format!("bit.lshift({a},{n})"),
                expected: oracle_lshift(a, n),
            });
            cases.push(Case {
                lua_call: format!("bit.rshift({a},{n})"),
                expected: oracle_rshift(a, n),
            });
        }
    }

    cases
}

// ---------------------------------------------------------------------------
// Script construction
// ---------------------------------------------------------------------------

/// Concatenate `BITWISE_FALLBACK` with one `print(...)` call per test case.
///
/// The polyfill defines `local bit = {}` which shadows `LuaJIT`'s native `bit`
/// library when running under `LuaJIT` — we are testing our implementation, not
/// the native one.
fn build_script(cases: &[Case]) -> String {
    let mut script = String::with_capacity(BITWISE_FALLBACK.len() + cases.len() * 32);
    script.push_str(BITWISE_FALLBACK);
    script.push('\n');
    for c in cases {
        script.push_str("print(");
        script.push_str(&c.lua_call);
        script.push_str(")\n");
    }
    script
}

// ---------------------------------------------------------------------------
// Process helpers
// ---------------------------------------------------------------------------

/// Return the name of the first `luajit`-compatible binary found in PATH,
/// or `None` if none is available.
fn find_luajit() -> Option<String> {
    for candidate in &["luajit", "luajit2"] {
        let found = Command::new(candidate)
            .args(["-e", "os.exit(0)"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if found {
            return Some((*candidate).to_owned());
        }
    }
    None
}

/// Write `script` to a temporary file, invoke `bin` on it, and return stdout.
///
/// The temp file is unconditionally removed after the process exits.
/// Uses the process ID in the filename to avoid collisions if the test suite
/// is run in parallel across multiple processes.
fn run_script(bin: &str, script: &str) -> Result<String, String> {
    let tmp = std::env::temp_dir().join(format!("valua_diff_{}.lua", std::process::id()));

    std::fs::write(&tmp, script).map_err(|e| format!("write temp script: {e}"))?;

    let result = Command::new(bin)
        .arg(&tmp)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let _ = std::fs::remove_file(&tmp);

    let out = result.map_err(|e| format!("spawn {bin}: {e}"))?;

    if !out.status.success() {
        return Err(format!(
            "luajit exited {status}\nstderr: {stderr}",
            status = out.status,
            stderr = String::from_utf8_lossy(&out.stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

/// Execute every `BITWISE_FALLBACK` operation under a real `luajit` process and
/// compare each output line against the Rust oracle.
///
/// Skips (without failing) when `luajit` is not installed.
#[test]
fn differential_bitwise_fallback() {
    let Some(bin) = find_luajit() else {
        eprintln!(
            "differential_bitwise_fallback: luajit not found in PATH — skipping.\n\
             Install luajit to enable this test (Linux: apt install luajit, \
             macOS: brew install luajit)."
        );
        return;
    };

    let cases = build_cases();
    let script = build_script(&cases);

    let raw = run_script(&bin, &script).expect("luajit execution failed");

    let lines: Vec<&str> = raw.lines().collect();

    assert_eq!(
        lines.len(),
        cases.len(),
        "output line count mismatch: expected {} lines (one per print call), got {}.\n\
         Script:\n{script}",
        cases.len(),
        lines.len(),
    );

    let mut failures: Vec<String> = Vec::new();

    for (i, (case, line)) in cases.iter().zip(lines.iter()).enumerate() {
        match line.trim().parse::<i64>() {
            Err(e) => {
                failures.push(format!(
                    "  [{i:>4}] print({lua}) → {line:?} — parse error: {e}",
                    lua = case.lua_call
                ));
            }
            Ok(got) if got != case.expected => {
                failures.push(format!(
                    "  [{i:>4}] print({lua}) → {got}  expected {exp}",
                    lua = case.lua_call,
                    exp = case.expected,
                ));
            }
            Ok(_) => {}
        }
    }

    assert!(
        failures.is_empty(),
        "{failed}/{total} differential cases failed under {bin}:\n{list}",
        failed = failures.len(),
        total = cases.len(),
        list = failures.join("\n"),
    );

    eprintln!(
        "differential_bitwise_fallback: {total} cases passed via {bin}",
        total = cases.len(),
    );
}
