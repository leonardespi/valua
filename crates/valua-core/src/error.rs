//! Top-level compile error that wraps errors from all pipeline stages.

use thiserror::Error;
use valua_codegen::CodeGenError;
use valua_diagnostics::Diagnostic;
use valua_parser::ParseError;
use valua_transformer::TransformError;

/// A compile error from any stage of the valua transpile pipeline.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("{} lint error(s) found", .0.len())]
    Lint(Vec<Diagnostic>),

    #[error("transform error: {0}")]
    Transform(#[from] TransformError),

    #[error("code generation error: {0}")]
    CodeGen(#[from] CodeGenError),
}

impl CompileError {
    /// If this is a parse error, convert it into a rich [`Diagnostic`].
    ///
    /// Returns `None` for non-parse errors; callers that need the diagnostic
    /// regardless of variant should match explicitly.
    pub fn into_parse_diagnostic(self) -> Option<Diagnostic> {
        match self {
            Self::Parse(e) => Some(e.into_diagnostic()),
            _ => None,
        }
    }
}
