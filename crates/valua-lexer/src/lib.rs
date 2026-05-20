// Lexer uses intentional patterns that trigger pedantic lints.
// cast_*: lexer converts byte offsets (usize/u64) to span fields (u32/i64) — casts are bounded by
//   source length which is i32::MAX max; the precision loss in u64→f64 is expected for float
//   literals (mirrors Lua's own conversion).
// match_same_arms / unnested_or_patterns / manual_let_else / single_match_else / bool_to_int_with_if:
//   logos-callback style produces these patterns; rewriting would sacrifice clarity.
// too_many_lines: the `next_token` dispatch function maps every Lua token; extraction would obscure
//   the grammar at a glance.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::unnested_or_patterns,
    clippy::manual_let_else,
    clippy::single_match_else,
    clippy::bool_to_int_with_if,
    clippy::elidable_lifetime_names
)]

use logos::Logos;
use valua_diagnostics::Span;

pub use error::LexError;
pub use token::Token;

mod error;
mod token;

// ── Line index ────────────────────────────────────────────────────────────────

struct LineIndex {
    /// Byte offset of the first character of each line (0-indexed lines).
    line_starts: Vec<usize>,
}

impl LineIndex {
    fn from_source(src: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// Returns 1-based (line, col) for a given byte offset.
    fn line_col(&self, byte_offset: usize) -> (u32, u32) {
        let line_idx = self.line_starts.partition_point(|&s| s <= byte_offset) - 1;
        let col = byte_offset - self.line_starts[line_idx];
        (line_idx as u32 + 1, col as u32 + 1)
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// A token paired with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

impl SpannedToken {
    #[must_use]
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}

/// Lexer that holds a reference to the source text and produces tokens.
pub struct Lexer<'src> {
    source: &'src str,
}

impl<'src> Lexer<'src> {
    #[must_use]
    pub fn new(source: &'src str) -> Self {
        Self { source }
    }

    /// Tokenize the entire source and return all tokens, including a final `Eof`.
    pub fn tokenize(&self) -> Result<Vec<SpannedToken>, LexError> {
        let line_index = LineIndex::from_source(self.source);
        let mut logos_lex = Token::lexer(self.source);
        let mut tokens: Vec<SpannedToken> = Vec::new();

        while let Some(result) = logos_lex.next() {
            let range = logos_lex.span();
            let (line, col) = line_index.line_col(range.start);
            let span = Span::new(range.start, range.end, line, col);

            match result {
                Ok(Token::Comment) => {
                    // skip_comment already consumed past the comment; never emitted
                }
                Ok(token) => {
                    tokens.push(SpannedToken::new(token, span));
                }
                Err(()) => {
                    return Err(match logos_lex.extras.last_error_kind {
                        token::LexErrorKind::UnterminatedString => {
                            LexError::UnterminatedString { span }
                        }
                        token::LexErrorKind::InvalidEscape => LexError::InvalidEscape { span },
                        token::LexErrorKind::UnterminatedLongString => {
                            LexError::UnterminatedLongString { span }
                        }
                        token::LexErrorKind::InvalidNumber => LexError::InvalidNumber { span },
                        token::LexErrorKind::UnexpectedChar => {
                            let ch = self.source[range.start..].chars().next().unwrap_or('\0');
                            LexError::UnexpectedChar { ch, span }
                        }
                    });
                }
            }
        }

        // TD5: unterminated long comment is silently consumed by skip_comment but flagged here.
        if logos_lex.extras.last_error_kind == token::LexErrorKind::UnterminatedLongString {
            let start = logos_lex.extras.last_error_start;
            let (line, col) = line_index.line_col(start);
            let span = Span::new(start, self.source.len(), line, col);
            return Err(LexError::UnterminatedLongString { span });
        }

        let eof_offset = self.source.len();
        let (eof_line, eof_col) = line_index.line_col(eof_offset.saturating_sub(1));
        let eof_span = Span::new(eof_offset, eof_offset, eof_line, eof_col);
        tokens.push(SpannedToken::new(Token::Eof, eof_span));

        Ok(tokens)
    }
}

// ── Iterator API (for future streaming use) ───────────────────────────────────

/// Iterator wrapper that yields tokens one at a time.
pub struct TokenIter<'src> {
    inner: Vec<SpannedToken>,
    pos: usize,
    #[allow(dead_code)]
    source: &'src str,
}

impl<'src> Iterator for TokenIter<'src> {
    type Item = Result<SpannedToken, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.inner.len() {
            let t = self.inner[self.pos].clone();
            self.pos += 1;
            Some(Ok(t))
        } else {
            None
        }
    }
}

impl<'src> IntoIterator for Lexer<'src> {
    type Item = Result<SpannedToken, LexError>;
    type IntoIter = TokenIter<'src>;

    fn into_iter(self) -> Self::IntoIter {
        let inner = self.tokenize().unwrap_or_default();
        TokenIter {
            inner,
            pos: 0,
            source: self.source,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> Vec<Token> {
        Lexer::new(src)
            .tokenize()
            .expect("lex failed")
            .into_iter()
            .map(|st| st.token)
            .collect()
    }

    #[test]
    fn test_lexes_integer() {
        let tokens = lex("42");
        assert_eq!(tokens[0], Token::Integer(42));
    }

    #[test]
    fn test_lexes_hex_integer() {
        let tokens = lex("0xFF");
        assert_eq!(tokens[0], Token::HexInteger(0xFF));
    }

    #[test]
    fn test_lexes_const_attribute() {
        // <const> is not a single token; it lexes as Lt, Ident("const"), Gt
        let tokens = lex("<const>");
        assert_eq!(tokens[0], Token::Lt);
        assert_eq!(tokens[1], Token::Ident("const".into()));
        assert_eq!(tokens[2], Token::Gt);
    }

    #[test]
    fn test_lexes_bitwise_operators() {
        let tokens = lex("& | ~ << >>");
        assert_eq!(tokens[0], Token::Ampersand);
        assert_eq!(tokens[1], Token::Pipe);
        assert_eq!(tokens[2], Token::Tilde);
        assert_eq!(tokens[3], Token::LtLt);
        assert_eq!(tokens[4], Token::GtGt);
    }

    #[test]
    fn test_lexes_integer_division() {
        let tokens = lex("a // b");
        assert_eq!(tokens[1], Token::DoubleSlash);
    }

    #[test]
    fn test_lex_error_unknown_char() {
        let result = Lexer::new("@").tokenize();
        assert!(matches!(result, Err(LexError::UnexpectedChar { .. })));
    }

    #[test]
    fn test_lex_error_unterminated_string() {
        let result = Lexer::new("\"hello").tokenize();
        assert!(matches!(result, Err(LexError::UnterminatedString { .. })));
    }

    #[test]
    fn test_lex_error_invalid_escape_hex_bad_digit() {
        // \xGG — G is not a hex digit
        let result = Lexer::new("\"\\xGG\"").tokenize();
        assert!(matches!(result, Err(LexError::InvalidEscape { .. })));
    }

    #[test]
    fn test_lex_error_invalid_escape_decimal_overflow() {
        // \256 — out of 0–255 range
        let result = Lexer::new("\"\\256\"").tokenize();
        assert!(matches!(result, Err(LexError::InvalidEscape { .. })));
    }

    #[test]
    fn test_lex_error_invalid_escape_unicode_bad_codepoint() {
        // \u{D800} — surrogate, not a valid Unicode scalar
        let result = Lexer::new("\"\\u{D800}\"").tokenize();
        assert!(matches!(result, Err(LexError::InvalidEscape { .. })));
    }

    #[test]
    fn test_lex_error_invalid_escape_unicode_empty_braces() {
        let result = Lexer::new("\"\\u{}\"").tokenize();
        assert!(matches!(result, Err(LexError::InvalidEscape { .. })));
    }

    #[test]
    fn test_lex_error_unterminated_long_string() {
        let result = Lexer::new("[[hello").tokenize();
        assert!(matches!(
            result,
            Err(LexError::UnterminatedLongString { .. })
        ));
    }

    #[test]
    fn test_lex_error_unterminated_long_comment() {
        let result = Lexer::new("--[[ unterminated").tokenize();
        assert!(matches!(
            result,
            Err(LexError::UnterminatedLongString { .. })
        ));
    }

    #[test]
    fn test_skips_line_comment() {
        let tokens = lex("1 -- comment\n2");
        assert_eq!(tokens[0], Token::Integer(1));
        assert_eq!(tokens[1], Token::Integer(2));
    }

    #[test]
    fn test_skips_long_comment() {
        let tokens = lex("1 --[[ multi\nline ]] 2");
        assert_eq!(tokens[0], Token::Integer(1));
        assert_eq!(tokens[1], Token::Integer(2));
    }

    #[test]
    fn test_lexes_string() {
        let tokens = lex(r#""hello""#);
        assert_eq!(tokens[0], Token::StringLit("hello".into()));
    }

    #[test]
    fn test_lexes_long_string() {
        let tokens = lex("[[hello world]]");
        assert_eq!(tokens[0], Token::StringLit("hello world".into()));
    }

    #[test]
    fn test_all_fixture_inputs_lex() {
        let fixtures = [
            include_str!("../../../tests/fixtures/bitwise_and/input.lua"),
            include_str!("../../../tests/fixtures/bitwise_or/input.lua"),
            include_str!("../../../tests/fixtures/bitwise_xor/input.lua"),
            include_str!("../../../tests/fixtures/bitwise_not/input.lua"),
            include_str!("../../../tests/fixtures/shift_left/input.lua"),
            include_str!("../../../tests/fixtures/shift_right/input.lua"),
            include_str!("../../../tests/fixtures/integer_division/input.lua"),
            include_str!("../../../tests/fixtures/const_attribute/input.lua"),
            include_str!("../../../tests/fixtures/close_attribute_simple/input.lua"),
            include_str!("../../../tests/fixtures/close_attribute_error_path/input.lua"),
            include_str!("../../../tests/fixtures/errors/E0101_math_type/input.lua"),
            include_str!("../../../tests/fixtures/errors/E0102_integer_overflow/input.lua"),
            include_str!("../../../tests/fixtures/errors/E0301_const_mutation/input.lua"),
            include_str!("../../../tests/fixtures/errors/E0401_post55_feature/input.lua"),
        ];
        for (i, src) in fixtures.iter().enumerate() {
            Lexer::new(src)
                .tokenize()
                .unwrap_or_else(|e| panic!("fixture[{i}] lex failed: {e}"));
        }
    }
}
