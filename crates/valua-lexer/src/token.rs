// See the module-level allow list in lib.rs for explanations.
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

use logos::{Filter, Logos};

// ── Error-tracking extras ─────────────────────────────────────────────────────
//
// `LexExtras` and `LexErrorKind` are implementation details of the logos
// callback mechanism and must not appear in the public API of valua-lexer.
//
// Module layout:
//   mod extras       — private; owns the type definitions (declared `pub` so
//                      the logos-generated `impl Logos for Token { type Extras
//                      = LexExtras }` satisfies Rust's E0446 visibility check,
//                      which fires only on explicitly-restricted qualifiers like
//                      `pub(crate)`, not on `pub` items in private modules)
//   use extras::…    — plain (non-pub) import into token scope; consumed by
//                      callbacks and the `#[logos(extras = LexExtras)]` derive
//   pub(crate) use   — re-exports `LexErrorKind` so lib.rs can match on it via
//                      `token::LexErrorKind::*`; `LexExtras` is NOT re-exported
//                      because lib.rs only accesses it through field projection
//                      (`logos_lex.extras.last_error_kind`), never by name

mod extras {
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub enum LexErrorKind {
        #[default]
        UnexpectedChar,
        UnterminatedString,
        InvalidEscape,
        UnterminatedLongString,
        InvalidNumber,
    }

    #[derive(Debug, Default)]
    pub struct LexExtras {
        pub last_error_kind: LexErrorKind,
        /// Byte offset where the error began (used for unterminated long comment detection).
        pub last_error_start: usize,
    }
}

// Private import — `LexExtras` only needed within token.rs for callbacks and
// the `#[logos(extras = LexExtras)]` derive; never re-exported.
use extras::LexExtras;

// `pub(crate) use` does double duty: it imports `LexErrorKind` into token scope
// (so callbacks can write `LexErrorKind::UnterminatedString` directly) AND
// re-exports it as `token::LexErrorKind` so lib.rs can match on its variants.
pub(crate) use extras::LexErrorKind;

// ── Comment skipping ──────────────────────────────────────────────────────────

fn skip_comment(lex: &mut logos::Lexer<Token>) -> Filter<()> {
    let remainder = lex.remainder();

    // Detect long comment: --[=*[
    if let Some(rest_after_bracket) = remainder.strip_prefix('[') {
        let level = rest_after_bracket
            .bytes()
            .take_while(|&b| b == b'=')
            .count();
        if rest_after_bracket.as_bytes().get(level) == Some(&b'[') {
            let open_consumed = 1 + level + 1; // '[' + '='*level + '['
            let close = format!("]{}]", "=".repeat(level));
            let inner = &remainder[open_consumed..];
            let bump = if let Some(pos) = inner.find(&close) {
                open_consumed + pos + close.len()
            } else {
                // Unterminated long comment — flag for post-loop error in tokenize().
                lex.extras.last_error_kind = LexErrorKind::UnterminatedLongString;
                lex.extras.last_error_start = lex.span().start;
                remainder.len()
            };
            lex.bump(bump);
            return Filter::Skip;
        }
    }

    // Short comment: consume to end of line
    let len = remainder.find('\n').unwrap_or(remainder.len());
    lex.bump(len);
    Filter::Skip
}

// ── Long-string literal ───────────────────────────────────────────────────────

fn lex_long_string(lex: &mut logos::Lexer<Token>) -> Option<String> {
    let slice = lex.slice(); // e.g. "[[" or "[=["
    let level = slice.len() - 2;
    let close = format!("]{}]", "=".repeat(level));
    let remainder = lex.remainder();

    // Lua spec: skip initial newline if present
    let skip = if remainder.starts_with("\r\n") {
        2
    } else if remainder.starts_with('\n') || remainder.starts_with('\r') {
        1
    } else {
        0
    };

    let content_src = &remainder[skip..];
    if let Some(pos) = content_src.find(&close) {
        let content = content_src[..pos].to_string();
        lex.bump(skip + pos + close.len());
        Some(content)
    } else {
        lex.extras.last_error_kind = LexErrorKind::UnterminatedLongString;
        None
    }
}

// ── Short string literals ─────────────────────────────────────────────────────

fn lex_double_string(lex: &mut logos::Lexer<Token>) -> Option<String> {
    scan_quoted_string(lex, '"')
}

fn lex_single_string(lex: &mut logos::Lexer<Token>) -> Option<String> {
    scan_quoted_string(lex, '\'')
}

fn scan_quoted_string(lex: &mut logos::Lexer<Token>, quote: char) -> Option<String> {
    // Default kind for EOF / bare-newline failures (set before any ? can fire).
    lex.extras.last_error_kind = LexErrorKind::UnterminatedString;
    let remainder = lex.remainder();
    let mut result = String::new();
    let mut iter = remainder.char_indices().peekable();

    loop {
        match iter.next() {
            None => return None, // unterminated — kind already set
            Some((_, '\n')) | Some((_, '\r')) => return None, // bare newline — kind already set
            Some((i, c)) if c == quote => {
                lex.bump(i + c.len_utf8());
                return Some(result);
            }
            Some((_, '\\')) => {
                // Backslash cut-off (EOF after \) — still UnterminatedString.
                let (_, esc) = iter.next()?;
                match esc {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    'a' => result.push('\x07'),
                    'b' => result.push('\x08'),
                    'f' => result.push('\x0C'),
                    'v' => result.push('\x0B'),
                    '\\' => result.push('\\'),
                    '\'' => result.push('\''),
                    '"' => result.push('"'),
                    '\n' => result.push('\n'),
                    '\r' => {
                        result.push('\n');
                        if iter.peek().map(|(_, c)| *c) == Some('\n') {
                            iter.next();
                        }
                    }
                    'z' => {
                        while let Some((_, c)) = iter.peek() {
                            if c.is_ascii_whitespace() {
                                iter.next();
                            } else {
                                break;
                            }
                        }
                    }
                    'x' => {
                        // \xXX — must be exactly two hex digits.
                        let h1 = match iter.next() {
                            Some((_, c)) if c.is_ascii_hexdigit() => c,
                            _ => {
                                lex.extras.last_error_kind = LexErrorKind::InvalidEscape;
                                return None;
                            }
                        };
                        let h2 = match iter.next() {
                            Some((_, c)) if c.is_ascii_hexdigit() => c,
                            _ => {
                                lex.extras.last_error_kind = LexErrorKind::InvalidEscape;
                                return None;
                            }
                        };
                        // Two valid hex chars; parse cannot fail.
                        let val = u8::from_str_radix(&format!("{h1}{h2}"), 16)
                            .expect("two hex chars always parse");
                        result.push(val as char);
                    }
                    d @ '0'..='9' => {
                        // \ddd — 1–3 decimal digits, value 0–255.
                        let mut num = d.to_string();
                        for _ in 0..2 {
                            if matches!(iter.peek().map(|(_, c)| *c), Some('0'..='9')) {
                                let (_, c) = iter.next().unwrap();
                                num.push(c);
                            } else {
                                break;
                            }
                        }
                        let val: u32 = match num.parse() {
                            Ok(v) => v,
                            Err(_) => {
                                lex.extras.last_error_kind = LexErrorKind::InvalidEscape;
                                return None;
                            }
                        };
                        if val > 255 {
                            lex.extras.last_error_kind = LexErrorKind::InvalidEscape;
                            return None;
                        }
                        // val ≤ 255, so char::from_u32 always succeeds.
                        result.push(char::from_u32(val).expect("val ≤ 255 is valid Unicode"));
                    }
                    'u' => {
                        // \u{XXXX} — braced hex Unicode codepoint.
                        if iter.next().map(|(_, c)| c) != Some('{') {
                            lex.extras.last_error_kind = LexErrorKind::InvalidEscape;
                            return None;
                        }
                        let mut hex = String::new();
                        loop {
                            match iter.next() {
                                Some((_, '}')) => break,
                                Some((_, c)) if c.is_ascii_hexdigit() => hex.push(c),
                                _ => {
                                    lex.extras.last_error_kind = LexErrorKind::InvalidEscape;
                                    return None;
                                }
                            }
                        }
                        if hex.is_empty() {
                            lex.extras.last_error_kind = LexErrorKind::InvalidEscape;
                            return None;
                        }
                        let val = match u32::from_str_radix(&hex, 16) {
                            Ok(v) => v,
                            Err(_) => {
                                lex.extras.last_error_kind = LexErrorKind::InvalidEscape;
                                return None;
                            }
                        };
                        match char::from_u32(val) {
                            Some(c) => result.push(c),
                            None => {
                                lex.extras.last_error_kind = LexErrorKind::InvalidEscape;
                                return None;
                            }
                        }
                    }
                    other => result.push(other), // unrecognized escape: pass through
                }
            }
            Some((_, c)) => result.push(c),
        }
    }
}

// ── Numeric literal helpers ───────────────────────────────────────────────────

fn parse_hex_int(lex: &mut logos::Lexer<Token>) -> Option<i64> {
    lex.extras.last_error_kind = LexErrorKind::InvalidNumber;
    let s = &lex.slice()[2..]; // strip 0x/0X
    i64::from_str_radix(s, 16)
        .ok()
        .or_else(|| u64::from_str_radix(s, 16).ok().map(|v| v as i64))
}

fn parse_dec_int(lex: &mut logos::Lexer<Token>) -> Option<i64> {
    lex.extras.last_error_kind = LexErrorKind::InvalidNumber;
    let s = lex.slice();
    s.parse::<i64>()
        .ok()
        .or_else(|| s.parse::<u64>().ok().map(|v| v as i64))
}

fn parse_dec_float(lex: &mut logos::Lexer<Token>) -> Option<f64> {
    lex.extras.last_error_kind = LexErrorKind::InvalidNumber;
    lex.slice().parse::<f64>().ok()
}

fn parse_hex_float(lex: &mut logos::Lexer<Token>) -> Option<f64> {
    lex.extras.last_error_kind = LexErrorKind::InvalidNumber;
    let s = lex.slice();
    parse_hex_float_str(s)
}

fn parse_hex_float_str(s: &str) -> Option<f64> {
    // Format: 0x[hex][.hex][p[+-]dec]
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))?;
    let (mantissa_str, exp_str) = if let Some(p) = s.find(['p', 'P']) {
        (&s[..p], Some(&s[p + 1..]))
    } else {
        (s, None)
    };
    let (int_part, frac_part) = if let Some(dot) = mantissa_str.find('.') {
        (&mantissa_str[..dot], Some(&mantissa_str[dot + 1..]))
    } else {
        (mantissa_str, None)
    };
    let int_val = if int_part.is_empty() {
        0u64
    } else {
        u64::from_str_radix(int_part, 16).ok()?
    };
    let mut mantissa = int_val as f64;
    if let Some(frac) = frac_part {
        let frac_val = u64::from_str_radix(frac, 16).ok()?;
        mantissa += frac_val as f64 / (16f64.powi(frac.len() as i32));
    }
    let exp: i32 = match exp_str {
        None => 0,
        Some(e) => e.parse().ok()?,
    };
    Some(mantissa * 2f64.powi(exp))
}

// ── Token enum ────────────────────────────────────────────────────────────────

/// Every terminal token recognised by the Lua 5.5 grammar (Lua 5.4 is a supported subset).
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\x0C\x0B]+")]
#[logos(extras = LexExtras)]
pub enum Token {
    // ── Comments (skipped) ───────────────────────────────────────────────────
    // Never emitted by `tokenize()` — the orchestration loop drops it.
    // Declared `pub` because it is a variant of a `pub` enum; `#[doc(hidden)]`
    // signals it is an implementation detail and prevents it from appearing in
    // generated documentation.
    #[doc(hidden)]
    #[token("--", skip_comment)]
    Comment,

    // ── Long string literals ─────────────────────────────────────────────────
    // Must be before operators so [[ is not lexed as two brackets.
    #[regex(r"\[=*\[", lex_long_string)]
    // ── Short string literals ────────────────────────────────────────────────
    #[token("\"", lex_double_string)]
    #[token("'", lex_single_string)]
    StringLit(String),

    // ── Numeric literals ─────────────────────────────────────────────────────
    // Hex float must precede hex int (longer match wins, but be explicit).
    #[regex(
        r"0[xX][0-9a-fA-F]*\.[0-9a-fA-F]+(?:[pP][+-]?[0-9]+)?",
        parse_hex_float
    )]
    #[regex(r"0[xX][0-9a-fA-F]+[pP][+-]?[0-9]+", parse_hex_float)]
    // Decimal floats
    #[regex(r"[0-9]+\.[0-9]+(?:[eE][+-]?[0-9]+)?", parse_dec_float)]
    #[regex(r"\.[0-9]+(?:[eE][+-]?[0-9]+)?", parse_dec_float)]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+", parse_dec_float)]
    Float(f64),

    // Hex int must come before decimal int.
    // Two distinct variants so the parser can propagate the notation hint to the AST.
    #[regex(r"0[xX][0-9a-fA-F]+", parse_hex_int)]
    HexInteger(i64),

    #[regex(r"[0-9]+", parse_dec_int)]
    Integer(i64),

    // ── Identifiers and keywords ─────────────────────────────────────────────
    // Keywords must be listed before Ident so exact-match takes priority on tie.
    #[token("and")]
    And,
    #[token("break")]
    Break,
    #[token("do")]
    Do,
    #[token("else")]
    Else,
    #[token("elseif")]
    Elseif,
    #[token("end")]
    End,
    #[token("false")]
    False,
    #[token("for")]
    For,
    #[token("function")]
    Function,
    #[token("goto")]
    Goto,
    #[token("if")]
    If,
    #[token("in")]
    In,
    #[token("local")]
    Local,
    #[token("nil")]
    Nil,
    #[token("not")]
    Not,
    #[token("or")]
    Or,
    #[token("repeat")]
    Repeat,
    #[token("return")]
    Return,
    #[token("then")]
    Then,
    #[token("true")]
    True,
    #[token("until")]
    Until,
    #[token("while")]
    While,

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // ── Attribute keywords (not lexed; matched by name in parser) ───────────
    AttrConst,
    AttrClose,

    // ── Multi-character operators (longer matches win over single-char) ───────
    #[token("==")]
    Eq,
    #[token("~=")]
    TildeEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("<<")]
    LtLt,
    #[token(">>")]
    GtGt,
    #[token("//")]
    DoubleSlash,
    #[token("::")]
    DblColon,
    #[token("..")]
    DotDot,
    #[token("...")]
    DotDotDot,

    // ── Single-character operators ───────────────────────────────────────────
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("^")]
    Caret,
    #[token("#")]
    Hash,
    #[token("&")]
    Ampersand,
    #[token("~")]
    Tilde,
    #[token("|")]
    Pipe,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("=")]
    Assign,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,

    // ── End of file (injected manually, never matched by logos) ─────────────
    Eof,
}

impl Token {
    /// Short human-readable name for use in diagnostic messages.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Integer(n) => format!("integer `{n}`"),
            Self::HexInteger(n) => format!("integer `0x{n:x}`"),
            Self::Float(f) => format!("float `{f}`"),
            Self::StringLit(s) => format!("string `{s:?}`"),
            Self::Ident(s) => format!("`{s}`"),
            Self::And => "`and`".into(),
            Self::Break => "`break`".into(),
            Self::Do => "`do`".into(),
            Self::Else => "`else`".into(),
            Self::Elseif => "`elseif`".into(),
            Self::End => "`end`".into(),
            Self::False => "`false`".into(),
            Self::For => "`for`".into(),
            Self::Function => "`function`".into(),
            Self::Goto => "`goto`".into(),
            Self::If => "`if`".into(),
            Self::In => "`in`".into(),
            Self::Local => "`local`".into(),
            Self::Nil => "`nil`".into(),
            Self::Not => "`not`".into(),
            Self::Or => "`or`".into(),
            Self::Repeat => "`repeat`".into(),
            Self::Return => "`return`".into(),
            Self::Then => "`then`".into(),
            Self::True => "`true`".into(),
            Self::Until => "`until`".into(),
            Self::While => "`while`".into(),
            Self::Eq => "`==`".into(),
            Self::TildeEq => "`~=`".into(),
            Self::LtEq => "`<=`".into(),
            Self::GtEq => "`>=`".into(),
            Self::LtLt => "`<<`".into(),
            Self::GtGt => "`>>`".into(),
            Self::DoubleSlash => "`//`".into(),
            Self::DblColon => "`::`".into(),
            Self::DotDot => "`..`".into(),
            Self::DotDotDot => "`...`".into(),
            Self::Plus => "`+`".into(),
            Self::Minus => "`-`".into(),
            Self::Star => "`*`".into(),
            Self::Slash => "`/`".into(),
            Self::Percent => "`%`".into(),
            Self::Caret => "`^`".into(),
            Self::Hash => "`#`".into(),
            Self::Ampersand => "`&`".into(),
            Self::Tilde => "`~`".into(),
            Self::Pipe => "`|`".into(),
            Self::Lt => "`<`".into(),
            Self::Gt => "`>`".into(),
            Self::Assign => "`=`".into(),
            Self::LParen => "`(`".into(),
            Self::RParen => "`)`".into(),
            Self::LBrace => "`{`".into(),
            Self::RBrace => "`}`".into(),
            Self::LBracket => "`[`".into(),
            Self::RBracket => "`]`".into(),
            Self::Semicolon => "`;`".into(),
            Self::Colon => "`:`".into(),
            Self::Comma => "`,`".into(),
            Self::Dot => "`.`".into(),
            Self::AttrConst => "`const`".into(),
            Self::AttrClose => "`close`".into(),
            Self::Comment => "comment".into(),
            Self::Eof => "end of file".into(),
        }
    }
}
