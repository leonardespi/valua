//! Code generator error type.

use thiserror::Error;
use valua_diagnostics::Span;

/// Errors produced by the Lua 5.1 code emitter.
#[derive(Debug, Error)]
pub enum CodeGenError {
    #[error("AST node at {span:?} cannot be represented in Lua {target}: {detail}")]
    UnsupportedNode { target: &'static str, detail: String, span: Span },

    #[error("internal emitter error: {detail}")]
    Internal { detail: String },
}
