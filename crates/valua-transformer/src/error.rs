//! Transformer error type.

use thiserror::Error;
use valua_diagnostics::Span;

/// Errors produced by AST transformation passes.
#[derive(Debug, Error)]
pub enum TransformError {
    #[error("assignment to <const> local '{name}' at {span:?}")]
    ConstViolation { name: String, span: Span },

    #[error("unsupported Lua 5.5 feature in pass '{pass}' at {span:?}: {detail}")]
    UnsupportedFeature {
        pass: &'static str,
        detail: String,
        span: Span,
    },

    #[error("internal transform error in pass '{pass}': {detail}")]
    Internal { pass: &'static str, detail: String },
}
