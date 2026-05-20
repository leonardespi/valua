//! Orchestrates the full valua transpile pipeline: lex → parse → transform → codegen.

use valua_codegen::{EmitOptions, LuaTarget};

pub use error::CompileError;

// Re-export public types consumers are likely to need.
pub use valua_ast::{Attribute, BinaryOp, Block, Expression, Statement, UnaryOp};
pub use valua_codegen::{EmitOptions as CodeGenOptions, LuaTarget as Target};
pub use valua_diagnostics::{Diagnostic, Severity, Span};
pub use valua_polyfills::FeatureSet;

mod error;

// ── Options ───────────────────────────────────────────────────────────────────

/// Options controlling a full compilation run.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Target Lua runtime.
    pub target: LuaTarget,
    /// Whether to inject polyfill preambles for detected features.
    pub inject_polyfills: bool,
    /// Code formatting options passed to the emitter.
    pub emit: EmitOptions,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: LuaTarget::Lua51,
            inject_polyfills: true,
            emit: EmitOptions::default(),
        }
    }
}

impl CompileOptions {
    /// Convenience builder targeting Lua 5.1.
    #[must_use]
    pub fn lua51() -> Self {
        Self::default()
    }

    /// Convenience builder targeting `LuaJIT`.
    #[must_use]
    pub fn luajit() -> Self {
        Self {
            target: LuaTarget::LuaJIT,
            emit: EmitOptions {
                target: LuaTarget::LuaJIT,
                emit_header_comment: false,
                ..EmitOptions::default()
            },
            ..Self::default()
        }
    }
}

// ── Compiler ──────────────────────────────────────────────────────────────────

/// The main entry point: runs the complete transpile pipeline and returns Lua 5.1 source.
pub struct Compiler;

impl Compiler {
    /// Transpile `source` (Lua 5.5) to Lua 5.1 using `opts`.
    ///
    /// Runs: parse → lint (abort on any Error-severity diagnostic) → transform → emit.
    /// Transform and emit are not yet implemented (Phase 3).
    ///
    /// # Errors
    /// Returns `CompileError` on any parse, lint, transform, or codegen failure.
    pub fn compile(source: &str, opts: CompileOptions) -> Result<String, CompileError> {
        use valua_diagnostics::Severity;
        use valua_lint::LintPipeline;

        use valua_codegen::{CodeGen, LuaEmitter};
        use valua_transformer::{
            BitwiseOpTransform, CloseAttributeTransform, ConstAttributeValidator, FeatureDetector,
            IntegerDivisionTransform, PolyfillInjector, TransformPipeline,
        };

        let mut block = valua_parser::parse(source)?;

        let lint_pipeline = LintPipeline::default_for(opts.target);
        let diags = lint_pipeline.run(&block);
        let errors: Vec<_> = diags
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        if !errors.is_empty() {
            return Err(CompileError::Lint(errors));
        }

        // Detect features before transformation — BitwiseOpTransform rewrites the ops away,
        // so detection must run against the original AST.
        let features = FeatureDetector::detect(&block);

        let mut pipeline = TransformPipeline::new();
        pipeline
            .add_pass(ConstAttributeValidator)
            .add_pass(BitwiseOpTransform)
            .add_pass(IntegerDivisionTransform)
            .add_pass(CloseAttributeTransform)
            .add_pass(PolyfillInjector::with_features_for_target(
                features,
                opts.target,
            ));
        pipeline.run(&mut block)?;

        let emitter = LuaEmitter::new(opts.emit);
        emitter.emit(&block).map_err(CompileError::CodeGen)
    }

    /// Parse `source` and return the AST without running any analysis or transformation.
    ///
    /// # Errors
    /// Returns `CompileError::Parse` on any lex or parse failure.
    pub fn parse_only(source: &str) -> Result<Block, CompileError> {
        valua_parser::parse(source).map_err(CompileError::Parse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_empty_source() {
        let result = Compiler::compile("", CompileOptions::default());
        assert_eq!(result.expect("empty source should compile"), "");
    }

    #[test]
    fn test_compile_options_default() {
        let opts = CompileOptions::default();
        assert_eq!(opts.target, Target::Lua51);
        assert!(opts.inject_polyfills);
        assert_eq!(CompileOptions::lua51().target, Target::Lua51);
    }

    #[test]
    fn test_parse_only_returns_block() {
        let block = Compiler::parse_only("local x = 1").expect("parse failed");
        assert_eq!(block.stmts.len(), 1);
        assert!(matches!(block.stmts[0], Statement::LocalDecl(_)));
    }
}
