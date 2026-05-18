//! Parse error type.

use thiserror::Error;
use valua_diagnostics::Span;
use valua_lexer::{LexError, Token};

/// Errors produced by the Lua 5.4 parser.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("lex error: {0}")]
    Lex(#[from] LexError),

    #[error("expected {expected:?} but found {found:?} at {span:?}")]
    Expected { expected: String, found: Token, span: Span },

    #[error("unexpected token {found:?} at {span:?}")]
    Unexpected { found: Token, span: Span },

    #[error("expression too complex — maximum nesting depth exceeded at {span:?}")]
    NestingDepthExceeded { span: Span },
}
