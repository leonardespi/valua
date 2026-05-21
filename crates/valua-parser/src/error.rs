//! Parse error type.

use thiserror::Error;
use valua_diagnostics::{Diagnostic, Span};
use valua_lexer::{LexError, Token};

/// Errors produced by the Lua 5.5 parser.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("lex error: {0}")]
    Lex(#[from] LexError),

    #[error("expected {expected}, found {}", found.describe())]
    Expected {
        expected: String,
        found: Token,
        span: Span,
    },

    #[error("unexpected token {}", found.describe())]
    Unexpected { found: Token, span: Span },

    #[error("expression too complex — maximum nesting depth exceeded")]
    NestingDepthExceeded { span: Span },
}

impl ParseError {
    /// Convert this parse error into a rich [`Diagnostic`] with an error code
    /// and source-level span suitable for caret rendering.
    ///
    /// Error code allocation (E02xx — structural syntax violations):
    /// - `E0201`: Unexpected / mismatched token.
    /// - `E0202`: Invalid attribute layout (unknown attribute name).
    /// - `E0203`: Expression nesting depth exceeded.
    #[must_use]
    pub fn into_diagnostic(self) -> Diagnostic {
        match self {
            Self::Lex(e) => e.into_diagnostic(),

            // E0202 — invalid attribute: parser expected "const or close"
            Self::Expected {
                ref expected,
                found: Token::Ident(ref name),
                span,
            } if expected == "const or close" => {
                Diagnostic::error(format!("unknown attribute `{name}`"), span)
                    .with_code("E0202")
                    .with_note(format!("`<{name}>` is not a valid Lua 5.4/5.5 attribute"))
                    .with_suggestion("valid attributes are `<const>` and `<close>`")
            }

            // E0201 — mismatched token
            Self::Expected {
                expected,
                ref found,
                span,
            } => Diagnostic::error(
                format!("expected {expected}, found {}", found.describe()),
                span,
            )
            .with_code("E0201"),

            // E0201 — unexpected token
            Self::Unexpected { ref found, span } => {
                Diagnostic::error(format!("unexpected token {}", found.describe()), span)
                    .with_code("E0201")
            }

            // E0203 — recursion guard
            Self::NestingDepthExceeded { span } => Diagnostic::error(
                "expression too complex — maximum nesting depth exceeded".to_string(),
                span,
            )
            .with_code("E0203"),
        }
    }
}
