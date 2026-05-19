use valua_ast::{Block, Call, Expression, Statement, TableField};
use valua_diagnostics::Diagnostic;

use crate::Lint;

/// Detects Lua 5.4 features that cannot be represented in LuaJIT or Lua 5.1.
///
/// - E0101: `math.type` — used as call or bare reference; observes integer/float distinction
///   absent in LuaJIT. Firing on bare references (`local f = math.type`) prevents indirect
///   use that bypasses the call-site check.
/// - E0102: integer literal at `i64::MAX` — overflow semantics differ from LuaJIT (IEEE 754)
/// - E0103: `math.tointeger` — used as call or bare reference; converts float→integer or
///   returns nil, which intrinsically observes the integer/float distinction. Does not exist
///   in LuaJIT.
pub struct UnsupportedFeatureGuard;

impl Lint for UnsupportedFeatureGuard {
    fn check(&self, block: &Block) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        check_block(block, &mut diags);
        diags
    }
}

fn check_block(block: &Block, diags: &mut Vec<Diagnostic>) {
    for stmt in &block.stmts {
        check_stmt(stmt, diags);
    }
}

fn check_stmt(stmt: &Statement, diags: &mut Vec<Diagnostic>) {
    match stmt {
        Statement::LocalDecl(d) => {
            for val in &d.values {
                check_expr(val, diags);
            }
        }
        Statement::Assign(a) => {
            for val in &a.values {
                check_expr(val, diags);
            }
        }
        Statement::Return(r) => {
            for val in &r.values {
                check_expr(val, diags);
            }
        }
        Statement::ExprStmt(e) => check_expr(e, diags),
        Statement::Do(b) => check_block(&b.body, diags),
        Statement::While(w) => {
            check_expr(&w.condition, diags);
            check_block(&w.body, diags);
        }
        Statement::Repeat(r) => {
            check_block(&r.body, diags);
            check_expr(&r.condition, diags);
        }
        Statement::If(i) => {
            check_expr(&i.condition, diags);
            check_block(&i.then_block, diags);
            for elseif in &i.elseif_clauses {
                check_expr(&elseif.condition, diags);
                check_block(&elseif.body, diags);
            }
            if let Some(else_block) = &i.else_block {
                check_block(else_block, diags);
            }
        }
        Statement::NumericFor(f) => {
            check_expr(&f.start, diags);
            check_expr(&f.limit, diags);
            if let Some(step) = &f.step {
                check_expr(step, diags);
            }
            check_block(&f.body, diags);
        }
        Statement::GenericFor(f) => {
            for iter in &f.iterators {
                check_expr(iter, diags);
            }
            check_block(&f.body, diags);
        }
        Statement::FunctionDecl(f) => check_block(&f.func.body, diags),
        Statement::LocalFunctionDecl(f) => check_block(&f.func.body, diags),
        _ => {}
    }
}

fn check_expr(expr: &Expression, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expression::Integer(val, span) if *val == i64::MAX => {
            diags.push(
                Diagnostic::error(
                    "integer overflow semantics differ between Lua 5.4 and LuaJIT",
                    *span,
                )
                .with_code("E0102")
                .with_suggestion(
                    "Use bit.* operations on the bit pattern explicitly, or accept double-precision semantics",
                ),
            );
        }
        Expression::Call(call) => check_call(call, diags),
        Expression::BinOp(lhs, _, rhs, _) => {
            check_expr(lhs, diags);
            check_expr(rhs, diags);
        }
        Expression::UnOp(_, operand, _) => check_expr(operand, diags),
        Expression::Index(base, field, span) => {
            // Bare references to math.type / math.tointeger (not inside a Call::Call func
            // position) are caught here. Call-position uses are caught in check_call, which
            // does not recurse into func when it matches, so there is no double-fire.
            if is_math_numeric_intrinsic(base, field) {
                diags.push(numeric_intrinsic_diagnostic(field, *span));
            } else {
                check_expr(base, diags);
            }
        }
        Expression::IndexExpr(base, key, _) => {
            check_expr(base, diags);
            check_expr(key, diags);
        }
        Expression::Function(f) => check_block(&f.body, diags),
        Expression::Table(t) => {
            for field in &t.fields {
                match field {
                    TableField::ExprKey { key, value, .. } => {
                        check_expr(key, diags);
                        check_expr(value, diags);
                    }
                    TableField::NameKey { value, .. } => check_expr(value, diags),
                    TableField::Positional(value) => check_expr(value, diags),
                }
            }
        }
        _ => {}
    }
}

fn check_call(call: &Call, diags: &mut Vec<Diagnostic>) {
    match call {
        Call::Call { func, args, .. } => {
            // Intercept math.type and math.tointeger at call site. Do not recurse into
            // func after matching — the bare-reference arm in check_expr handles the
            // Index node when it is not in call position, so avoiding recursion here
            // prevents double-firing on the same span.
            if let Expression::Index(base, field, span) = func.as_ref() {
                if is_math_numeric_intrinsic(base, field) {
                    diags.push(numeric_intrinsic_diagnostic(field, *span));
                } else {
                    check_expr(func, diags);
                }
            } else {
                check_expr(func, diags);
            }
            for arg in args {
                check_expr(arg, diags);
            }
        }
        Call::MethodCall { obj, args, .. } => {
            check_expr(obj, diags);
            for arg in args {
                check_expr(arg, diags);
            }
        }
    }
}

/// Returns true when `base.field` is a numeric type-introspection function that
/// has no equivalent in LuaJIT — currently `math.type` and `math.tointeger`.
fn is_math_numeric_intrinsic(base: &Expression, field: &str) -> bool {
    matches!(base, Expression::Name(n, _) if n == "math") && matches!(field, "type" | "tointeger")
}

/// Build the appropriate diagnostic for a `math.type` or `math.tointeger` use.
fn numeric_intrinsic_diagnostic(field: &str, span: valua_diagnostics::Span) -> Diagnostic {
    match field {
        "type" => Diagnostic::error(
            "math.type observes the integer/float distinction absent in LuaJIT",
            span,
        )
        .with_code("E0101")
        .with_suggestion(
            "Remove type discrimination or track numeric kind explicitly in user data",
        ),
        "tointeger" => Diagnostic::error(
            "math.tointeger observes the integer/float distinction absent in LuaJIT",
            span,
        )
        .with_code("E0103")
        .with_suggestion(
            "Replace with math.floor() if truncation is intended, or restructure to avoid integer/float discrimination",
        ),
        _ => unreachable!("numeric_intrinsic_diagnostic called with unrecognised field"),
    }
}

#[cfg(test)]
mod tests {
    use valua_diagnostics::{CollectingReporter, Reporter};

    use super::UnsupportedFeatureGuard;
    use crate::Lint;

    fn parse(src: &str) -> valua_ast::Block {
        valua_parser::parse(src).expect("parse failed")
    }

    fn codes(src: &str) -> Vec<&'static str> {
        UnsupportedFeatureGuard
            .check(&parse(src))
            .into_iter()
            .filter_map(|d| d.code)
            .collect()
    }

    // ── E0101: math.type detection ───────────────────────────────────────────

    #[test]
    fn uf_detects_math_type_call() {
        assert_eq!(codes("math.type(1)"), vec!["E0101"]);
    }

    #[test]
    fn uf_no_false_positive_math_floor() {
        assert!(codes("math.floor(1.5)").is_empty());
    }

    #[test]
    fn uf_no_false_positive_other_table_type() {
        assert!(codes("x.type(1)").is_empty());
    }

    #[test]
    fn uf_detects_math_type_in_local_decl() {
        assert_eq!(codes("local t = math.type(1)"), vec!["E0101"]);
    }

    #[test]
    fn uf_detects_math_type_in_return() {
        assert_eq!(codes("return math.type(x)"), vec!["E0101"]);
    }

    #[test]
    fn uf_detects_math_type_nested_in_table_value() {
        let src = "local t = { a = math.type(1) }";
        assert_eq!(codes(src), vec!["E0101"]);
    }

    #[test]
    fn uf_detects_math_type_as_multiple_args() {
        let src = "foo(math.type(1), math.type(2))";
        let result = codes(src);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|&c| c == "E0101"));
    }

    #[test]
    fn uf_detects_math_type_in_function_body() {
        let src = "local function f() return math.type(1) end";
        assert_eq!(codes(src), vec!["E0101"]);
    }

    // ── E0102: i64::MAX literal detection ───────────────────────────────────

    #[test]
    fn uf_detects_i64_max_literal() {
        let src = format!("local x = {}", i64::MAX);
        assert_eq!(codes(&src), vec!["E0102"]);
    }

    #[test]
    fn uf_no_false_positive_i64_max_minus_one() {
        let src = format!("local x = {}", i64::MAX - 1);
        assert!(codes(&src).is_empty());
    }

    #[test]
    fn uf_no_false_positive_small_integer() {
        assert!(codes("local x = 42").is_empty());
    }

    #[test]
    fn uf_detects_i64_max_in_positional_table() {
        let src = format!("local t = {{ {} }}", i64::MAX);
        assert_eq!(codes(&src), vec!["E0102"]);
    }

    #[test]
    fn uf_detects_i64_max_in_function_body() {
        let src = format!("function f() return {} end", i64::MAX);
        assert_eq!(codes(&src), vec!["E0102"]);
    }

    // ── E0101: math.type as bare reference (not call) ────────────────────────

    #[test]
    fn uf_detects_math_type_bare_reference_in_local() {
        // `local f = math.type` — stores function reference without calling it.
        // Must fire E0101 to prevent indirect use that bypasses call-site check.
        assert_eq!(codes("local f = math.type"), vec!["E0101"]);
    }

    #[test]
    fn uf_detects_math_type_bare_reference_as_argument() {
        // Passing math.type as an argument to another function.
        assert_eq!(codes("foo(math.type)"), vec!["E0101"]);
    }

    #[test]
    fn uf_no_false_positive_math_floor_reference() {
        // math.floor as a reference — not a restricted intrinsic.
        assert!(codes("local f = math.floor").is_empty());
    }

    // ── E0103: math.tointeger detection ─────────────────────────────────────

    #[test]
    fn uf_detects_math_tointeger_call() {
        assert_eq!(codes("math.tointeger(1.5)"), vec!["E0103"]);
    }

    #[test]
    fn uf_detects_math_tointeger_in_local_decl() {
        assert_eq!(codes("local n = math.tointeger(x)"), vec!["E0103"]);
    }

    #[test]
    fn uf_detects_math_tointeger_in_return() {
        assert_eq!(codes("return math.tointeger(x)"), vec!["E0103"]);
    }

    #[test]
    fn uf_detects_math_tointeger_in_function_body() {
        let src = "local function f() return math.tointeger(1.0) end";
        assert_eq!(codes(src), vec!["E0103"]);
    }

    #[test]
    fn uf_detects_math_tointeger_bare_reference() {
        assert_eq!(codes("local f = math.tointeger"), vec!["E0103"]);
    }

    #[test]
    fn uf_no_false_positive_tointeger_on_other_table() {
        // x.tointeger is not math.tointeger.
        assert!(codes("x.tointeger(1)").is_empty());
    }

    // ── Combined E0101 + E0102 ───────────────────────────────────────────────

    #[test]
    fn uf_detects_both_codes_in_same_block() {
        let src = format!("math.type(1)\nlocal x = {}", i64::MAX);
        let result = codes(&src);
        assert!(result.contains(&"E0101"), "expected E0101 in {result:?}");
        assert!(result.contains(&"E0102"), "expected E0102 in {result:?}");
    }

    // ── Combined E0101 + E0103 ───────────────────────────────────────────────

    #[test]
    fn uf_detects_e0101_and_e0103_in_same_block() {
        let src = "math.type(1)\nmath.tointeger(2.5)";
        let result = codes(src);
        assert!(result.contains(&"E0101"), "expected E0101 in {result:?}");
        assert!(result.contains(&"E0103"), "expected E0103 in {result:?}");
    }

    // ── CollectingReporter integration ───────────────────────────────────────

    #[test]
    fn uf_collecting_reporter_captures_e0101() {
        let src = "math.type(1)";
        let diags = UnsupportedFeatureGuard.check(&parse(src));
        let mut reporter = CollectingReporter::default();
        for d in &diags {
            reporter.report(d, src, "test.lua");
        }
        assert!(reporter.has_errors());
        assert_eq!(reporter.diagnostics[0].code, Some("E0101"));
    }

    #[test]
    fn uf_collecting_reporter_captures_e0102() {
        let src = format!("local x = {}", i64::MAX);
        let diags = UnsupportedFeatureGuard.check(&parse(&src));
        let mut reporter = CollectingReporter::default();
        for d in &diags {
            reporter.report(d, &src, "test.lua");
        }
        assert!(reporter.has_errors());
        assert_eq!(reporter.diagnostics[0].code, Some("E0102"));
    }

    #[test]
    fn uf_clean_code_no_errors() {
        let src = "local x = 42\nreturn x + 1";
        let diags = UnsupportedFeatureGuard.check(&parse(src));
        assert!(diags.is_empty());
    }
}
