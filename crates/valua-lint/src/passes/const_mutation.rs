use std::collections::HashMap;

use valua_ast::{
    Assign, Attribute, Block, Call, Expression, FunctionBody, LocalDecl, Statement, TableField,
};
use valua_diagnostics::{Diagnostic, Span};

use crate::Lint;

/// E0301 — detects reassignment of `<const>`-attributed local variables.
pub struct ConstMutation;

impl Lint for ConstMutation {
    fn check(&self, block: &Block) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let mut scopes: Vec<ScopeFrame> = Vec::new();
        check_block(block, &mut scopes, &mut diags);
        diags
    }
}

// `Some(span)` → name is `<const>`, declared at `span`.
// `None`       → name is in scope but NOT const (shadows any outer const).
type ScopeFrame = HashMap<String, Option<Span>>;

fn check_block(block: &Block, scopes: &mut Vec<ScopeFrame>, diags: &mut Vec<Diagnostic>) {
    scopes.push(HashMap::new());
    for stmt in &block.stmts {
        check_stmt(stmt, scopes, diags);
    }
    scopes.pop();
}

fn check_function_body(
    func: &FunctionBody,
    scopes: &mut Vec<ScopeFrame>,
    diags: &mut Vec<Diagnostic>,
) {
    scopes.push(HashMap::new());
    // Parameters shadow any outer const of the same name.
    if let Some(scope) = scopes.last_mut() {
        for param in &func.params {
            scope.insert(param.name.clone(), None);
        }
    }
    for stmt in &func.body.stmts {
        check_stmt(stmt, scopes, diags);
    }
    scopes.pop();
}

fn check_stmt(stmt: &Statement, scopes: &mut Vec<ScopeFrame>, diags: &mut Vec<Diagnostic>) {
    match stmt {
        Statement::LocalDecl(decl) => {
            // Walk RHS expressions for closures BEFORE registering names so that
            // `local x <const> = 1; local f = function() x = 2 end` can see outer x.
            for val in &decl.values {
                check_expr_closures(val, scopes, diags);
            }
            register_locals(decl, scopes);
        }
        Statement::Assign(assign) => {
            for val in &assign.values {
                check_expr_closures(val, scopes, diags);
            }
            check_assign(assign, scopes, diags);
        }
        Statement::ExprStmt(e) => check_expr_closures(e, scopes, diags),
        Statement::Return(r) => {
            for val in &r.values {
                check_expr_closures(val, scopes, diags);
            }
        }
        Statement::Do(b) => check_block(&b.body, scopes, diags),
        Statement::While(w) => {
            check_expr_closures(&w.condition, scopes, diags);
            check_block(&w.body, scopes, diags);
        }
        Statement::Repeat(r) => {
            check_block(&r.body, scopes, diags);
            check_expr_closures(&r.condition, scopes, diags);
        }
        Statement::If(i) => {
            check_expr_closures(&i.condition, scopes, diags);
            check_block(&i.then_block, scopes, diags);
            for elseif in &i.elseif_clauses {
                check_expr_closures(&elseif.condition, scopes, diags);
                check_block(&elseif.body, scopes, diags);
            }
            if let Some(else_block) = &i.else_block {
                check_block(else_block, scopes, diags);
            }
        }
        Statement::NumericFor(f) => {
            check_expr_closures(&f.start, scopes, diags);
            check_expr_closures(&f.limit, scopes, diags);
            if let Some(step) = &f.step {
                check_expr_closures(step, scopes, diags);
            }
            check_block(&f.body, scopes, diags);
        }
        Statement::GenericFor(f) => {
            for iter in &f.iterators {
                check_expr_closures(iter, scopes, diags);
            }
            check_block(&f.body, scopes, diags);
        }
        Statement::FunctionDecl(f) => check_function_body(&f.func, scopes, diags),
        Statement::LocalFunctionDecl(f) => check_function_body(&f.func, scopes, diags),
        _ => {}
    }
}

fn check_expr_closures(
    expr: &Expression,
    scopes: &mut Vec<ScopeFrame>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expression::Function(f) => check_function_body(f, scopes, diags),
        Expression::Call(call) => match call {
            Call::Call { func, args, .. } => {
                check_expr_closures(func, scopes, diags);
                for arg in args {
                    check_expr_closures(arg, scopes, diags);
                }
            }
            Call::MethodCall { obj, args, .. } => {
                check_expr_closures(obj, scopes, diags);
                for arg in args {
                    check_expr_closures(arg, scopes, diags);
                }
            }
        },
        Expression::Table(t) => {
            for field in &t.fields {
                match field {
                    TableField::ExprKey { key, value, .. } => {
                        check_expr_closures(key, scopes, diags);
                        check_expr_closures(value, scopes, diags);
                    }
                    TableField::NameKey { value, .. } => {
                        check_expr_closures(value, scopes, diags);
                    }
                    TableField::Positional(value) => {
                        check_expr_closures(value, scopes, diags);
                    }
                }
            }
        }
        Expression::BinOp(lhs, _, rhs, _) => {
            check_expr_closures(lhs, scopes, diags);
            check_expr_closures(rhs, scopes, diags);
        }
        Expression::UnOp(_, operand, _) => check_expr_closures(operand, scopes, diags),
        Expression::Index(base, _, _) => check_expr_closures(base, scopes, diags),
        Expression::IndexExpr(base, key, _) => {
            check_expr_closures(base, scopes, diags);
            check_expr_closures(key, scopes, diags);
        }
        _ => {}
    }
}

fn register_locals(decl: &LocalDecl, scopes: &mut [ScopeFrame]) {
    if let Some(scope) = scopes.last_mut() {
        for name in &decl.names {
            let entry = if name.attribute == Some(Attribute::Const) {
                Some(name.span)
            } else {
                None
            };
            scope.insert(name.name.clone(), entry);
        }
    }
}

fn check_assign(assign: &Assign, scopes: &[ScopeFrame], diags: &mut Vec<Diagnostic>) {
    for target in &assign.targets {
        if let Expression::Name(name, mutation_span) = target {
            if let Some(decl_span) = find_const(scopes, name) {
                diags.push(
                    Diagnostic::error(
                        format!("assignment to const variable `{name}`"),
                        *mutation_span,
                    )
                    .with_code("E0301")
                    .with_secondary_label(decl_span, "declared as const here")
                    .with_note(format!(
                        "`{name}` was declared with <const>; Lua 5.4 treats this as a compile-time constant"
                    ))
                    .with_suggestion(
                        "Remove the assignment or remove the <const> attribute from the declaration",
                    ),
                );
            }
        }
    }
}

fn find_const(scopes: &[ScopeFrame], name: &str) -> Option<Span> {
    for scope in scopes.iter().rev() {
        match scope.get(name) {
            Some(Some(span)) => return Some(*span),
            Some(None) => return None, // inner non-const shadows outer const
            None => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use valua_diagnostics::{CollectingReporter, Reporter};

    use super::ConstMutation;
    use crate::Lint;

    fn parse(src: &str) -> valua_ast::Block {
        valua_parser::parse(src).expect("parse failed")
    }

    fn diags(src: &str) -> Vec<valua_diagnostics::Diagnostic> {
        ConstMutation.check(&parse(src))
    }

    fn codes(src: &str) -> Vec<&'static str> {
        diags(src).into_iter().filter_map(|d| d.code).collect()
    }

    // ── Basic detection ──────────────────────────────────────────────────────

    #[test]
    fn cm_detects_simple_mutation() {
        assert_eq!(codes("local x <const> = 1\nx = 2"), vec!["E0301"]);
    }

    #[test]
    fn cm_no_false_positive_plain_local() {
        assert!(codes("local x = 1\nx = 2").is_empty());
    }

    #[test]
    fn cm_no_false_positive_read_only() {
        assert!(codes("local x <const> = 1\nlocal y = x + 1").is_empty());
    }

    #[test]
    fn cm_multiple_mutations_same_const() {
        let result = codes("local x <const> = 1\nx = 2\nx = 3");
        assert_eq!(result, vec!["E0301", "E0301"]);
    }

    // ── Scope nesting ────────────────────────────────────────────────────────

    #[test]
    fn cm_mutation_in_do_block() {
        assert_eq!(
            codes("local x <const> = 1\ndo\n  x = 2\nend"),
            vec!["E0301"]
        );
    }

    #[test]
    fn cm_inner_non_const_shadows_outer_const() {
        // Bug 1 regression: inner `local x` must shadow outer `<const> x` → 0 diags
        assert!(codes("local x <const> = 1\ndo\n  local x = 2\n  x = 3\nend").is_empty());
    }

    #[test]
    fn cm_shadow_ends_at_block_boundary() {
        // After `do...end`, outer const is visible again → 1 diag for `x = 4`
        let result = codes("local x <const> = 1\ndo\n  local x = 2\n  x = 3\nend\nx = 4");
        assert_eq!(result, vec!["E0301"]);
    }

    #[test]
    fn cm_deeply_nested_do_blocks_detect_mutation() {
        let src = "local x <const> = 1\ndo\n  do\n    do\n      x = 2\n    end\n  end\nend";
        assert_eq!(codes(src), vec!["E0301"]);
    }

    #[test]
    fn cm_deeply_nested_shadow_stops_at_boundary() {
        // Non-const shadow at innermost level; after block exits, outer const is back
        let src = "local x <const> = 1\ndo\n  do\n    local x = 2\n    x = 3\n  end\n  x = 4\nend";
        // `x = 3` is shadowed → no diag; `x = 4` sees outer const → E0301
        assert_eq!(codes(src), vec!["E0301"]);
    }

    // ── Closures in values ────────────────────────────────────────────────────

    #[test]
    fn cm_mutation_in_anonymous_function_value() {
        // Bug 2 regression: closure captures outer const → should detect mutation
        let src = "local x <const> = 1\nlocal f = function() x = 2 end";
        assert_eq!(codes(src), vec!["E0301"]);
    }

    #[test]
    fn cm_mutation_in_named_function_decl() {
        let src = "local x <const> = 1\nfunction foo() x = 2 end";
        assert_eq!(codes(src), vec!["E0301"]);
    }

    #[test]
    fn cm_mutation_in_local_function_decl() {
        let src = "local x <const> = 1\nlocal function foo() x = 2 end";
        assert_eq!(codes(src), vec!["E0301"]);
    }

    // ── Parameter shadows ────────────────────────────────────────────────────

    #[test]
    fn cm_function_param_shadows_outer_const() {
        // Bug 3 regression: param `x` shadows outer const `x` → 0 diags inside body
        let src = "local x <const> = 1\nlocal function f(x) x = 2 end";
        assert!(codes(src).is_empty());
    }

    #[test]
    fn cm_function_param_does_not_affect_outer_scope() {
        // Outer const still visible and flagged after function declaration
        let src = "local x <const> = 1\nlocal function f(x) x = 2 end\nx = 3";
        assert_eq!(codes(src), vec!["E0301"]);
    }

    // ── Close attribute ──────────────────────────────────────────────────────

    #[test]
    fn cm_close_attribute_not_flagged() {
        // `<close>` is NOT `<const>` — mutations are allowed
        let src = "local x <close> = io.open('f')\nx = nil";
        assert!(codes(src).is_empty());
    }

    // ── Span verification ────────────────────────────────────────────────────

    #[test]
    fn cm_mutation_span_on_correct_line() {
        let src = "local x <const> = 1\nx = 2";
        let diagnostics = diags(src);
        assert_eq!(diagnostics.len(), 1);
        let d = &diagnostics[0];
        assert_eq!(d.code, Some("E0301"));
        assert_eq!(d.span.line, 2, "mutation span should be on line 2");
    }

    #[test]
    fn cm_secondary_label_on_declaration_line() {
        let src = "local x <const> = 1\nx = 2";
        let diagnostics = diags(src);
        assert_eq!(diagnostics.len(), 1);
        let d = &diagnostics[0];
        assert_eq!(d.secondary_labels.len(), 1);
        assert_eq!(
            d.secondary_labels[0].0.line, 1,
            "secondary label should point to line 1 (declaration)"
        );
    }

    // ── If/while/repeat/for bodies ───────────────────────────────────────────

    #[test]
    fn cm_mutation_in_while_body() {
        let src = "local x <const> = 1\nwhile true do\n  x = 2\nend";
        assert_eq!(codes(src), vec!["E0301"]);
    }

    #[test]
    fn cm_mutation_in_if_then() {
        let src = "local x <const> = 1\nif true then\n  x = 2\nend";
        assert_eq!(codes(src), vec!["E0301"]);
    }

    #[test]
    fn cm_mutation_in_numeric_for_body() {
        let src = "local x <const> = 1\nfor i = 1, 10 do\n  x = 2\nend";
        assert_eq!(codes(src), vec!["E0301"]);
    }

    // ── CollectingReporter integration ───────────────────────────────────────

    #[test]
    fn cm_collecting_reporter_captures_e0301() {
        let src = "local x <const> = 1\nx = 2";
        let diagnostics = ConstMutation.check(&parse(src));
        let mut reporter = CollectingReporter::default();
        for d in &diagnostics {
            reporter.report(d, src, "test.lua");
        }
        assert!(reporter.has_errors());
        assert_eq!(reporter.diagnostics.len(), 1);
        assert_eq!(reporter.diagnostics[0].code, Some("E0301"));
    }

    #[test]
    fn cm_collecting_reporter_clean_code() {
        let src = "local x <const> = 1\nreturn x";
        let diagnostics = ConstMutation.check(&parse(src));
        let mut reporter = CollectingReporter::default();
        for d in &diagnostics {
            reporter.report(d, src, "test.lua");
        }
        assert!(!reporter.has_errors());
    }
}
