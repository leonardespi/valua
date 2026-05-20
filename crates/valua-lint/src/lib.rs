//! Static analysis for valua. Public crate — API stable from 1.0.
//!
//! Core usage:
//! ```ignore
//! let block = valua_parser::parse(src)?;
//! let diags = LintPipeline::default_for(valua_ast::LuaTarget::LuaJIT).run(&block);
//! ```

use valua_ast::{Block, LuaTarget};
use valua_diagnostics::Diagnostic;

pub mod passes;

use passes::const_mutation::ConstMutation;
use passes::unsupported::UnsupportedFeatureGuard;

/// A single static-analysis pass over a parsed AST.
pub trait Lint {
    fn check(&self, block: &Block) -> Vec<Diagnostic>;
}

/// Ordered sequence of lint passes applied to a parsed block.
pub struct LintPipeline {
    passes: Vec<Box<dyn Lint>>,
}

impl LintPipeline {
    /// Default lint pipeline for `target`.
    ///
    /// - `ConstMutation` (E0301) runs for all targets.
    /// - `UnsupportedFeatureGuard` (E0101, E0102) runs for Lua 5.1 and `LuaJIT`,
    ///   which both lack `math.type` and differ in integer overflow semantics.
    #[must_use]
    pub fn default_for(target: LuaTarget) -> Self {
        let mut passes: Vec<Box<dyn Lint>> = vec![Box::new(ConstMutation)];
        match target {
            LuaTarget::Lua51 | LuaTarget::LuaJIT => {
                passes.push(Box::new(UnsupportedFeatureGuard));
            }
        }
        Self { passes }
    }

    /// Run all passes and return all diagnostics in pass order.
    #[must_use]
    pub fn run(&self, block: &Block) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for pass in &self.passes {
            out.extend(pass.check(block));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use valua_ast::{Block, LuaTarget};
    use valua_diagnostics::{CollectingReporter, Diagnostic, Reporter, Span};

    use super::{Lint, LintPipeline};

    fn parse(src: &str) -> Block {
        valua_parser::parse(src).expect("parse failed")
    }

    fn dummy_span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    // ── Lint trait: mock implementations ─────────────────────────────────────

    struct AlwaysErrors;
    impl Lint for AlwaysErrors {
        fn check(&self, _: &Block) -> Vec<Diagnostic> {
            vec![Diagnostic::error("always fires", dummy_span()).with_code("E9999")]
        }
    }

    struct NeverErrors;
    impl Lint for NeverErrors {
        fn check(&self, _: &Block) -> Vec<Diagnostic> {
            vec![]
        }
    }

    #[test]
    fn lint_trait_mock_fires() {
        let block = parse("local x = 1");
        let diags = AlwaysErrors.check(&block);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Some("E9999"));
    }

    #[test]
    fn lint_trait_mock_silent() {
        let block = parse("local x = 1");
        assert!(NeverErrors.check(&block).is_empty());
    }

    // ── LintPipeline: target routing ─────────────────────────────────────────

    #[test]
    fn pipeline_lua51_runs_unsupported_guard() {
        let block = parse("math.type(1)");
        let diags = LintPipeline::default_for(LuaTarget::Lua51).run(&block);
        assert!(diags.iter().any(|d| d.code == Some("E0101")));
    }

    #[test]
    fn pipeline_luajit_runs_unsupported_guard() {
        let block = parse("math.type(1)");
        let diags = LintPipeline::default_for(LuaTarget::LuaJIT).run(&block);
        assert!(diags.iter().any(|d| d.code == Some("E0101")));
    }

    #[test]
    fn pipeline_all_targets_run_const_mutation() {
        let block = parse("local x <const> = 1\nx = 2");
        for target in [LuaTarget::Lua51, LuaTarget::LuaJIT] {
            let diags = LintPipeline::default_for(target).run(&block);
            assert!(
                diags.iter().any(|d| d.code == Some("E0301")),
                "{target:?} should detect E0301"
            );
        }
    }

    #[test]
    fn pipeline_run_returns_diags_in_pass_order() {
        let src = format!(
            "local x <const> = 1\nx = 2\nmath.type(1)\nlocal y = {}",
            i64::MAX
        );
        let diags = LintPipeline::default_for(LuaTarget::Lua51).run(&parse(&src));
        let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
        // E0301 from ConstMutation runs before E0101/E0102 from UnsupportedFeatureGuard
        let e0301_pos = codes.iter().position(|&c| c == "E0301").unwrap();
        let e0101_pos = codes.iter().position(|&c| c == "E0101").unwrap();
        assert!(
            e0301_pos < e0101_pos,
            "E0301 should appear before E0101 (pass order)"
        );
        assert!(codes.contains(&"E0102"));
    }

    #[test]
    fn pipeline_clean_code_no_diags() {
        let block = parse("local x = 1\nlocal y = x + 2\nreturn y");
        let diags = LintPipeline::default_for(LuaTarget::Lua51).run(&block);
        assert!(diags.is_empty());
    }

    // ── CollectingReporter integration ────────────────────────────────────────

    #[test]
    fn pipeline_with_collecting_reporter() {
        let src = "local x <const> = 1\nx = 2";
        let diags = LintPipeline::default_for(LuaTarget::Lua51).run(&parse(src));
        let mut reporter = CollectingReporter::default();
        for d in &diags {
            reporter.report(d, src, "test.lua");
        }
        assert!(reporter.has_errors());
        assert!(reporter.diagnostics.iter().any(|d| d.code == Some("E0301")));
    }

    #[test]
    fn pipeline_no_errors_reporter_clean() {
        let src = "local x = 1";
        let diags = LintPipeline::default_for(LuaTarget::Lua51).run(&parse(src));
        let mut reporter = CollectingReporter::default();
        for d in &diags {
            reporter.report(d, src, "test.lua");
        }
        assert!(!reporter.has_errors());
    }
}
