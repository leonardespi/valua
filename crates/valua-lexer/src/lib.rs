//! Lua 5.4 tokenizer — produces a flat `Vec<SpannedToken>` from source text.

use valua_diagnostics::Span;

pub use error::LexError;
pub use token::Token;

mod error;
mod token;

/// A token paired with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

impl SpannedToken {
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}

/// Lexer that holds a reference to the source text and produces tokens.
///
/// Backed by the `logos` crate — the `logos::Lexer` is stored inside so that
/// the actual tokenisation can be filled in without changing the public API.
pub struct Lexer<'src> {
    source: &'src str,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer over `source`.
    pub fn new(source: &'src str) -> Self {
        Self { source }
    }

    /// Tokenize the entire source and return all tokens.
    ///
    /// # Errors
    /// Returns a `LexError` on the first unrecognised character or malformed
    /// string/number literal.
    pub fn tokenize(&self) -> Result<Vec<SpannedToken>, LexError> {
        // TODO: drive logos::Lexer<Token> over self.source, collecting SpannedTokens
        todo!("drive logos lexer over source and collect SpannedToken vec")
    }
}

/// Iterator wrapper produced by `Lexer::into_iter`.
///
/// Yields `Result<SpannedToken, LexError>` so callers can handle errors lazily.
pub struct TokenIter<'src> {
    #[allow(dead_code)]
    source: &'src str,
    #[allow(dead_code)]
    pos: usize,
}

impl<'src> Iterator for TokenIter<'src> {
    type Item = Result<SpannedToken, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: advance logos lexer one token at a time
        todo!("advance logos lexer one step")
    }
}

impl<'src> IntoIterator for Lexer<'src> {
    type Item = Result<SpannedToken, LexError>;
    type IntoIter = TokenIter<'src>;

    fn into_iter(self) -> Self::IntoIter {
        TokenIter { source: self.source, pos: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "TODO: test_lexes_integer — '42' produces Token::Integer(42)"]
    fn test_lexes_integer() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_lexes_const_attribute — '<const>' produces the correct tokens"]
    fn test_lexes_const_attribute() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_lexes_bitwise_operators — '&|~<<>>' produces the correct tokens"]
    fn test_lexes_bitwise_operators() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_lexes_integer_division — '//' produces Token::DoubleSlash"]
    fn test_lexes_integer_division() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_lex_error_unknown_char — invalid char produces LexError"]
    fn test_lex_error_unknown_char() {
        todo!()
    }
}
