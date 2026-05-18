//! Integration test harness for valua.
//! See tests/fixtures/README.md for fixture format.
//! See docs/FIXTURE_ORCHESTRATOR.md for the workflow that produces fixtures.

use std::fs;
use std::path::{Path, PathBuf};
use valua_core::{CompileOptions, Compiler};

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
        if matches!(name.as_str(), "errors" | "backends") {
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
        let code = name.split('_').next().map(String::from).expect("code prefix");
        if !is_valid_error_code(&code) {
            panic!(
                "error fixture {} has invalid code prefix `{}`",
                name, code
            );
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

// ── Success fixture tests ─────────────────────────────────────────────────────

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_bitwise_and() {
    let root = Path::new(FIXTURE_ROOT).join("bitwise_and");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_bitwise_or() {
    let root = Path::new(FIXTURE_ROOT).join("bitwise_or");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_bitwise_xor() {
    let root = Path::new(FIXTURE_ROOT).join("bitwise_xor");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_bitwise_not() {
    let root = Path::new(FIXTURE_ROOT).join("bitwise_not");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_shift_left() {
    let root = Path::new(FIXTURE_ROOT).join("shift_left");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_shift_right() {
    let root = Path::new(FIXTURE_ROOT).join("shift_right");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_integer_division() {
    let root = Path::new(FIXTURE_ROOT).join("integer_division");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_const_attribute() {
    let root = Path::new(FIXTURE_ROOT).join("const_attribute");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_close_attribute_simple() {
    let root = Path::new(FIXTURE_ROOT).join("close_attribute_simple");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_close_attribute_error_path() {
    let root = Path::new(FIXTURE_ROOT).join("close_attribute_error_path");
    let input = read_file(&root.join("input.lua"));
    let expected = normalize(&read_file(&root.join("expected.lua")));
    let actual = Compiler::compile(&input, CompileOptions::luajit()).expect("compile failed");
    assert_eq!(normalize(&actual), expected);
}

// ── Error fixture tests ───────────────────────────────────────────────────────

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_e0101_math_type() {
    let root = Path::new(FIXTURE_ROOT).join("errors").join("E0101_math_type");
    let input = read_file(&root.join("input.lua"));
    let result = Compiler::compile(&input, CompileOptions::luajit());
    assert!(result.is_err(), "expected E0101 compile error");
    // TODO: render diagnostic via ConsoleReporter and compare against expected.txt
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_e0102_integer_overflow() {
    let root = Path::new(FIXTURE_ROOT)
        .join("errors")
        .join("E0102_integer_overflow");
    let input = read_file(&root.join("input.lua"));
    let result = Compiler::compile(&input, CompileOptions::luajit());
    assert!(result.is_err(), "expected E0102 compile error");
    // TODO: render diagnostic via ConsoleReporter and compare against expected.txt
}

#[test]
#[ignore = "Compiler::compile not yet implemented"]
fn fixture_e0301_const_mutation() {
    let root = Path::new(FIXTURE_ROOT)
        .join("errors")
        .join("E0301_const_mutation");
    let input = read_file(&root.join("input.lua"));
    let result = Compiler::compile(&input, CompileOptions::luajit());
    assert!(result.is_err(), "expected E0301 compile error");
    // TODO: render diagnostic via ConsoleReporter and compare against expected.txt
}

#[test]
#[ignore = "forward-looking: post-Lua-5.5 source support not yet in scope for 1.0; see PRD §14 Pivote A"]
fn fixture_e0401_post55_feature() {
    let root = Path::new(FIXTURE_ROOT)
        .join("errors")
        .join("E0401_post55_feature");
    let input = read_file(&root.join("input.lua"));
    let result = Compiler::compile(&input, CompileOptions::luajit());
    assert!(result.is_err(), "expected E0401 compile error");
    // TODO: render diagnostic via ConsoleReporter and compare against expected.txt
}
