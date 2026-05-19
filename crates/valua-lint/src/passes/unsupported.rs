use valua_ast::{Block, Call, Expression, Statement, TableField};
use valua_diagnostics::Diagnostic;

use crate::Lint;

/// Detects Lua 5.4 features that cannot be represented in LuaJIT or Lua 5.1.
///
/// - E0101: `math.type()` calls — observes integer/float distinction absent in LuaJIT
/// - E0102: integer literal at `i64::MAX` — overflow semantics differ from LuaJIT (IEEE 754)
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
        Expression::Index(base, _, _) => check_expr(base, diags),
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
            if is_math_type(func) {
                diags.push(
                    Diagnostic::error(
                        "reflection of numerical types is not representable in LuaJIT target",
                        func.span(),
                    )
                    .with_code("E0101")
                    .with_suggestion(
                        "Remove the type discrimination or refactor to track numeric kind in user code",
                    ),
                );
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

fn is_math_type(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Index(base, field, _)
            if matches!(base.as_ref(), Expression::Name(n, _) if n == "math")
               && field == "type"
    )
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

    // ── Combined E0101 + E0102 ───────────────────────────────────────────────

    #[test]
    fn uf_detects_both_codes_in_same_block() {
        let src = format!("math.type(1)\nlocal x = {}", i64::MAX);
        let result = codes(&src);
        assert!(result.contains(&"E0101"), "expected E0101 in {result:?}");
        assert!(result.contains(&"E0102"), "expected E0102 in {result:?}");
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
