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
    /// Convenience builder targeting LuaJIT.
    pub fn luajit() -> Self {
        Self { target: LuaTarget::LuaJIT, emit: EmitOptions { target: LuaTarget::LuaJIT, ..EmitOptions::default() }, ..Self::default() }
    }
}

// ── Compiler ──────────────────────────────────────────────────────────────────

/// The main entry point: runs the complete transpile pipeline and returns Lua 5.1 source.
pub struct Compiler;

impl Compiler {
    /// Transpile `source` (Lua 5.4) to Lua 5.1 using `opts`.
    ///
    /// # Errors
    /// Returns `CompileError` on any lex, parse, transform, or codegen failure.
    pub fn compile(_source: &str, _opts: CompileOptions) -> Result<String, CompileError> {
        // TODO: 1. lex via valua_lexer::Lexer
        // TODO: 2. parse via valua_parser::parse
        // TODO: 3. build and run default_pipeline (or custom) via valua_transformer
        // TODO: 4. emit via LuaEmitter::new(_opts.emit).emit(&block)
        todo!("drive lex → parse → transform → codegen pipeline")
    }

    /// Parse only — useful for syntax checking without transformation.
    ///
    /// # Errors
    /// Returns `CompileError::Parse` on any lex or parse failure.
    pub fn parse_only(_source: &str) -> Result<Block, CompileError> {
        // TODO: lex + parse, return Block without transforming
        todo!("lex and parse without transformation")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "TODO: test_compile_empty_source — empty string produces empty output"]
    fn test_compile_empty_source() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_compile_options_default — default options target Lua 5.1"]
    fn test_compile_options_default() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_parse_only_returns_block — parse_only produces a Block"]
    fn test_parse_only_returns_block() {
        todo!()
    }
}
