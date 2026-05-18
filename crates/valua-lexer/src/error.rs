//! Lexer error type.

use thiserror::Error;
use valua_diagnostics::Span;

/// Errors produced by the Lua 5.4 lexer.
#[derive(Debug, Error)]
pub enum LexError {
    #[error("unexpected character '{ch}' at {span:?}")]
    UnexpectedChar { ch: char, span: Span },

    #[error("unterminated string literal starting at {span:?}")]
    UnterminatedString { span: Span },

    #[error("invalid escape sequence in string literal at {span:?}")]
    InvalidEscape { span: Span },

    #[error("invalid numeric literal at {span:?}")]
    InvalidNumber { span: Span },

    #[error("unterminated long string/comment starting at {span:?}")]
    UnterminatedLongString { span: Span },
}
