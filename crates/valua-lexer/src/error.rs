//! Lexer error type.

use thiserror::Error;
use valua_diagnostics::{Diagnostic, Span};

/// Errors produced by the Lua 5.5 lexer.
#[derive(Debug, Error)]
pub enum LexError {
    #[error("unexpected character '{ch}'")]
    UnexpectedChar { ch: char, span: Span },

    #[error("unterminated string literal")]
    UnterminatedString { span: Span },

    #[error("invalid escape sequence in string literal")]
    InvalidEscape { span: Span },

    #[error("invalid numeric literal")]
    InvalidNumber { span: Span },

    #[error("unterminated long string or comment")]
    UnterminatedLongString { span: Span },
}

impl LexError {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::UnexpectedChar { span, .. }
            | Self::UnterminatedString { span }
            | Self::InvalidEscape { span }
            | Self::InvalidNumber { span }
            | Self::UnterminatedLongString { span } => *span,
        }
    }

    #[must_use]
    pub fn into_diagnostic(self) -> Diagnostic {
        let span = self.span();
        Diagnostic::error(self.to_string(), span)
    }
}
