//! Integration test harness for valua.
//! See tests/fixtures/README.md for fixture format.
//! See docs/FIXTURE_ORCHESTRATOR.md for the workflow that produces fixtures.

// Test utilities use patterns that trigger pedantic lints; suppress them here rather than
// scattering individual allows across hundreds of test functions.
#![allow(
    clippy::unreadable_literal,    // exact integer literals in fixtures (e.g. math.maxinteger)
    clippy::uninlined_format_args, // clearer to keep args separate in panics/assertions
    clippy::manual_assert,         // if !x { panic!(...) } is fine in test helpers
    clippy::redundant_closure_for_method_calls, // closures in iterators are often more readable
    clippy::unnecessary_join,      // explicit join clearer than alternative
    clippy::doc_markdown,          // test comments don't need backtick discipline
    clippy::cast_possible_wrap,    // span field conversions bounded by source length
    clippy::map_unwrap_or,         // map().unwrap_or() readable in test contexts
)]

use std::fs;
use std::path::{Path, PathBuf};
use valua_core::{CompileOptions, Compiler};
use valua_diagnostics::render_diagnostic_to_string;

const FIXTURE_ROOT: &str = "tests/fixtures";

fn is_valid_error_code(code: &str) -> bool {
    let bytes = code.as_bytes();
    bytes.len() == 5
        && (bytes[0] == b'E' || bytes[0] == b'W')
        && bytes[1..].iter().all(u8::is_ascii_digit)
}

fn discover_success_fixtures() -> Vec<(String, PathBuf)> {
    let root = Path::new(FIXTURE_ROOT);
    let mut out = Vec::new();
    for entry in fs::read_dir(root).expect("fixture root must exist") {
        let entry = entry.expect("readable entry");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(String::from)
            .expect("UTF-8 name");
        if matches!(name.as_str(), "errors" | "backends" | "lua51") {
            continue;
        }
        let input = path.join("input.lua");
        let expected = path.join("expected.lua");
        if !input.exists() {
            panic!("fixture {} missing input.lua", name);
        }
        if !expected.exists() {
            panic!("fixture {} missing expected.lua", name);
        }
        out.push((name, path));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn discover_error_fixtures() -> Vec<(String, String, PathBuf)> {
    let root = Path::new(FIXTURE_ROOT).join("errors");
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    for entry in fs::read_dir(&root).expect("errors dir readable") {
        let entry = entry.expect("readable entry");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(String::from)
            .expect("UTF-8 name");
        let code = name
            .split('_')
            .next()
            .map(String::from)
            .expect("code prefix");
        if !is_valid_error_code(&code) {
            panic!("error fixture {} has invalid code prefix `{}`", name, code);
        }
        let input = path.join("input.lua");
        let expected = path.join("expected.txt");
        if !input.exists() {
            panic!("error fixture {} missing input.lua", name);
        }
        if !expected.exists() {
            panic!("error fixture {} missing expected.txt", name);
        }
        out.push((name, code, path));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Strip leading verification tag comments and blank lines, trailing whitespace
/// per line, and trailing blank lines. Appends a single trailing newline.
fn normalize(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();

    // Skip leading VERIFIED/INFERRED/CONJECTURE/Evidence tag lines.
    let mut start = 0;
    while start < lines.len() {
        let trimmed = lines[start].trim_start();
        if trimmed.starts_with("-- VERIFIED:")
            || trimmed.starts_with("-- INFERRED:")
            || trimmed.starts_with("-- CONJECTURE:")
            || trimmed.starts_with("-- Evidence:")
        {
            start += 1;
        } else {
            break;
        }
    }

    let remaining = &lines[start..];

    // Skip leading blank lines after tags.
    let first_content = remaining
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(remaining.len());
    let remaining = &remaining[first_content..];

    // Drop trailing blank lines.
    let last_content = remaining
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map(|i| i + 1)
        .unwrap_or(0);
    let remaining = &remaining[..last_content];

    // Strip trailing whitespace from each line.
    let normalized: Vec<String> = remaining.iter().map(|l| l.trim_end().to_string()).collect();

    let mut out = normalized.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

fn read_file(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("cannot read {}", path.display()))
        .replace("\r\n", "\n")
}

// ── Decimal-emission helpers ──────────────────────────────────────────────────

/// Returns true if `line` contains a non-decimal integer literal (`0x…` / `0b…`)
/// outside of a Lua line comment. Conservative: also fires inside string literals,
/// which is acceptable — fixture expected files must not contain hex string content.
fn has_non_decimal_int_literal(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("--") {
        return false;
    }
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'0'
            && matches!(bytes[i + 1], b'x' | b'X' | b'b' | b'B')
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric())
        {
            return true;
        }
        i += 1;
    }
    false
}

/// Returns the leading VERIFIED/INFERRED/CONJECTURE/Evidence tag comment lines.
fn extract_header_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .take_while(|l| {
            let t = l.trim_start();
            t.starts_with("-- VERIFIED:")
                || t.starts_with("-- INFERRED:")
                || t.starts_with("-- CONJECTURE:")
                || t.starts_with("-- Evidence:")
        })
        .map(|s| s.to_string())
        .collect()
}

// ── Meta-tests ────────────────────────────────────────────────────────────────

#[test]
fn meta_all_fixtures_discoverable() {
    // Confirms discovery logic does not panic on a structurally valid fixture tree.
    let _success = discover_success_fixtures();
    let _errors = discover_error_fixtures();
}

#[test]
fn meta_error_fixtures_have_valid_codes() {
    for (name, code, _) in discover_error_fixtures() {
        assert!(
            is_valid_error_code(&code),
            "error fixture {} has invalid code `{}`",
            name,
            code
        );
    }
}

#[test]
fn meta_no_hex_in_success_fixture_expected_files() {
    let mut violations: Vec<String> = Vec::new();
    for (name, path) in discover_success_fixtures() {
        let expected_path = path.join("expected.lua");
        let content = read_file(&expected_path);
        for (line_no, line) in content.lines().enumerate() {
            if has_non_decimal_int_literal(line) {
                violations.push(format!("{}:{}: {}", name, line_no + 1, line.trim()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Decimal emission contract violated in expected.lua files:\n{}",
        violations.join("\n")
    );
}

#[test]
#[ignore = "snapshot regen: run with UPDATE_FIXTURES=1 cargo test update_all_fixtures -- --ignored"]
fn update_all_fixtures() {
    assert_eq!(
        std::env::var("UPDATE_FIXTURES").unwrap_or_default(),
        "1",
        "Set UPDATE_FIXTURES=1 to run this test"
    );

    let mut changed: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for (name, path) in discover_success_fixtures() {
        let input_path = path.join("input.lua");
        let expected_path = path.join("expected.lua");
        let input = read_file(&input_path);
        let existing = fs::read_to_string(&expected_path).unwrap_or_default();
        let headers = extract_header_lines(&existing);

        match Compiler::compile(&input, CompileOptions::luajit()) {
            Ok(output) => {
                let mut new_content = String::new();
                for h in &headers {
                    new_content.push_str(h);
                    new_content.push('\n');
                }
                if !headers.is_empty() {
                    new_content.push('\n');
                }
                new_content.push_str(&output);
                if new_content != existing {
                    fs::write(&expected_path, &new_content).unwrap_or_else(|e| {
                        panic!("cannot write {}: {}", expected_path.display(), e)
                    });
                    changed.push(name);
                }
            }
            Err(e) => failed.push(format!("{}: {}", name, e)),
        }
    }

    if changed.is_empty() {
        println!("All fixtures up to date.");
    } else {
        println!("Updated {} fixture(s):", changed.len());
        for f in &changed {
            println!("  - {}", f);
        }
    }

    assert!(
        failed.is_empty(),
        "Fixtures failed to compile during regeneration:\n{}",
        failed.join("\n")
    );
}

// ── Success fixture tests ─────────────────────────────────────────────────────

#[test]
fn fixture_bitwise_and() {
    let root = Path::new(FIXTURE_ROOT).join("bitwise_and");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_bitwise_or() {
    let root = Path::new(FIXTURE_ROOT).join("bitwise_or");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_bitwise_xor() {
    let root = Path::new(FIXTURE_ROOT).join("bitwise_xor");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_bitwise_not() {
    let root = Path::new(FIXTURE_ROOT).join("bitwise_not");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_shift_left() {
    let root = Path::new(FIXTURE_ROOT).join("shift_left");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_shift_right() {
    let root = Path::new(FIXTURE_ROOT).join("shift_right");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_integer_division() {
    let root = Path::new(FIXTURE_ROOT).join("integer_division");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_const_attribute() {
    let root = Path::new(FIXTURE_ROOT).join("const_attribute");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_close_attribute_simple() {
    let root = Path::new(FIXTURE_ROOT).join("close_attribute_simple");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_close_attribute_error_path() {
    let root = Path::new(FIXTURE_ROOT).join("close_attribute_error_path");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
fn fixture_close_attribute_multi() {
    let root = Path::new(FIXTURE_ROOT).join("close_attribute_multi");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

// ── Lua 5.1 target fixtures ───────────────────────────────────────────────────

#[test]
fn fixture_bitwise_and_lua51() {
    let root = Path::new(FIXTURE_ROOT).join("lua51").join("bitwise_and");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::lua51()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

// ── Error fixture tests ───────────────────────────────────────────────────────

fn render_lint_diagnostics(source: &str, err: valua_core::CompileError) -> String {
    let valua_core::CompileError::Lint(diags) = err else {
        panic!("expected CompileError::Lint, got something else");
    };
    diags
        .iter()
        .map(|d| render_diagnostic_to_string(d, source, "input.lua"))
        .collect::<Vec<_>>()
        .join("")
}

#[test]
fn fixture_e0103_math_tointeger() {
    let root = Path::new(FIXTURE_ROOT)
        .join("errors")
        .join("E0103_math_tointeger");
    let input = read_file(&root.join("input.lua"));
    let expected = read_file(&root.join("expected.txt"));
    let err = Compiler::compile(&input, CompileOptions::luajit())
        .expect_err("expected E0103 compile error");
    let rendered = render_lint_diagnostics(&input, err);
    pretty_assertions::assert_eq!(rendered, expected);
}

#[test]
fn fixture_e0101_math_type() {
    let root = Path::new(FIXTURE_ROOT)
        .join("errors")
        .join("E0101_math_type");
    let input = read_file(&root.join("input.lua"));
    let expected = read_file(&root.join("expected.txt"));
    let err = Compiler::compile(&input, CompileOptions::luajit())
        .expect_err("expected E0101 compile error");
    let rendered = render_lint_diagnostics(&input, err);
    pretty_assertions::assert_eq!(rendered, expected);
}

#[test]
fn fixture_e0102_integer_overflow() {
    let root = Path::new(FIXTURE_ROOT)
        .join("errors")
        .join("E0102_integer_overflow");
    let input = read_file(&root.join("input.lua"));
    let expected = read_file(&root.join("expected.txt"));
    let err = Compiler::compile(&input, CompileOptions::luajit())
        .expect_err("expected E0102 compile error");
    let rendered = render_lint_diagnostics(&input, err);
    pretty_assertions::assert_eq!(rendered, expected);
}

#[test]
fn fixture_e0301_const_mutation() {
    let root = Path::new(FIXTURE_ROOT)
        .join("errors")
        .join("E0301_const_mutation");
    let input = read_file(&root.join("input.lua"));
    let expected = read_file(&root.join("expected.txt"));
    let err = Compiler::compile(&input, CompileOptions::luajit())
        .expect_err("expected E0301 compile error");
    let rendered = render_lint_diagnostics(&input, err);
    pretty_assertions::assert_eq!(rendered, expected);
}

#[test]
fn fixture_e0401_post55_feature() {
    let root = Path::new(FIXTURE_ROOT)
        .join("errors")
        .join("E0401_post55_feature");
    let input = read_file(&root.join("input.lua"));
    let err = Compiler::compile(&input, CompileOptions::luajit())
        .expect_err("expected E0401 compile error");
    // Unknown attribute is a parse error (not a lint diagnostic).
    assert!(
        matches!(err, valua_core::CompileError::Parse(_)),
        "expected CompileError::Parse for unknown attribute, got: {err:?}"
    );
    // Error message must name the unrecognized attribute.
    assert!(
        err.to_string().contains("future_attr"),
        "error should identify the unknown attribute: {err}"
    );
}

// ── UV2: Precedence round-trip adversarial fuzzing ────────────────────────────

mod prec_fuzz {
    use valua_ast::{BinaryOp, Block, Expression, Return, Statement, UnaryOp};
    use valua_codegen::{CodeGen, LuaEmitter};
    use valua_diagnostics::Span;

    // Linear congruential generator — no external crate; deterministic across runs.
    struct Lcg(u64);
    impl Lcg {
        fn new(seed: u64) -> Self {
            Self(seed)
        }
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0
        }
        fn pick(&mut self, n: usize) -> usize {
            (self.next() >> 33) as usize % n
        }
    }

    const BINOPS: &[BinaryOp] = &[
        BinaryOp::Add,
        BinaryOp::Sub,
        BinaryOp::Mul,
        BinaryOp::Div,
        BinaryOp::Mod,
        BinaryOp::Pow, // right-assoc, highest prec
        BinaryOp::IDiv,
        BinaryOp::Lt,
        BinaryOp::Le,
        BinaryOp::Gt,
        BinaryOp::Ge,
        BinaryOp::Eq,
        BinaryOp::Ne,
        BinaryOp::And,
        BinaryOp::Or,
        BinaryOp::BitwiseAnd,
        BinaryOp::BitwiseOr,
        BinaryOp::BitwiseXor, // `~` same token as unary BitwiseNot
        BinaryOp::Shl,
        BinaryOp::Shr,
        BinaryOp::Concat, // right-assoc
    ];

    const UNOPS: &[UnaryOp] = &[
        UnaryOp::Neg,
        UnaryOp::Not,
        UnaryOp::Len,
        UnaryOp::BitwiseNot,
    ];

    const NAMES: &[&str] = &["a", "b", "c", "x", "y"];

    fn sp() -> Span {
        Span::dummy()
    }

    fn gen_expr(rng: &mut Lcg, depth: u32) -> Expression {
        // At depth 0, or with 1/3 probability at any depth, emit a leaf.
        if depth == 0 || rng.pick(3) == 0 {
            return gen_leaf(rng);
        }
        if rng.pick(2) == 0 {
            let op = BINOPS[rng.pick(BINOPS.len())];
            Expression::BinOp(
                Box::new(gen_expr(rng, depth - 1)),
                op,
                Box::new(gen_expr(rng, depth - 1)),
                sp(),
            )
        } else {
            let op = UNOPS[rng.pick(UNOPS.len())];
            Expression::UnOp(op, Box::new(gen_expr(rng, depth - 1)), sp())
        }
    }

    fn gen_leaf(rng: &mut Lcg) -> Expression {
        if rng.pick(2) == 0 {
            // Non-negative integers only: the parser always produces
            // UnOp(Neg, Integer(n)) for negative literals, never Integer(-n),
            // which would cause a spurious structural mismatch.
            Expression::Integer(rng.pick(100) as i64, sp())
        } else {
            Expression::Name(NAMES[rng.pick(NAMES.len())].to_string(), sp())
        }
    }

    fn wrap_return(expr: Expression) -> Block {
        Block {
            stmts: vec![Statement::Return(Return {
                values: vec![expr],
                span: sp(),
            })],
            span: sp(),
        }
    }

    fn expr_eq(a: &Expression, b: &Expression) -> bool {
        match (a, b) {
            (Expression::Integer(va, _), Expression::Integer(vb, _)) => va == vb,
            (Expression::Name(na, _), Expression::Name(nb, _)) => na == nb,
            (Expression::BinOp(la, oa, ra, _), Expression::BinOp(lb, ob, rb, _)) => {
                oa == ob && expr_eq(la, lb) && expr_eq(ra, rb)
            }
            (Expression::UnOp(oa, ea, _), Expression::UnOp(ob, eb, _)) => {
                oa == ob && expr_eq(ea, eb)
            }
            _ => false,
        }
    }

    #[test]
    fn prec_roundtrip_adversarial_1000() {
        let emitter = LuaEmitter::luajit();
        let mut failures: Vec<String> = Vec::new();

        for seed in 0u64..1000 {
            let mut rng = Lcg::new(
                seed.wrapping_mul(0x9e3779b97f4a7c15)
                    .wrapping_add(0x6c62272e07bb0142),
            );
            let expr = gen_expr(&mut rng, 4);
            let block = wrap_return(expr.clone());

            let lua_src = match emitter.emit(&block) {
                Ok(s) => s,
                Err(e) => {
                    failures.push(format!("seed {seed}: emit error: {e}"));
                    continue;
                }
            };

            let parsed_block = match valua_parser::parse(&lua_src) {
                Ok(b) => b,
                Err(e) => {
                    failures.push(format!("seed {seed}: parse error: {e}\n  lua: {lua_src}"));
                    continue;
                }
            };

            let parsed_expr = match parsed_block.stmts.as_slice() {
                [Statement::Return(r)] if r.values.len() == 1 => &r.values[0],
                other => {
                    failures.push(format!(
                        "seed {seed}: unexpected structure: {other:?}\n  lua: {lua_src}"
                    ));
                    continue;
                }
            };

            if !expr_eq(&expr, parsed_expr) {
                failures.push(format!(
                    "seed {seed}: round-trip mismatch\n  original: {expr:?}\n  parsed:   {parsed_expr:?}\n  lua:      {lua_src}"
                ));
            }
        }

        assert!(
            failures.is_empty(),
            "{} failure(s):\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}
