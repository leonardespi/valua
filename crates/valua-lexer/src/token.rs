//! `Token` enum covering every terminal symbol in Lua 5.4.

// NOTE: logos attribute macros will be added here when the lexer is implemented.
// For now the enum is declared without the derive so it compiles as a plain type.

/// Every terminal token recognised by the Lua 5.4 grammar.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Literals ─────────────────────────────────────────────────────────────
    Integer(i64),
    Float(f64),
    StringLit(String),

    // ── Identifiers ──────────────────────────────────────────────────────────
    Ident(String),

    // ── Keywords ─────────────────────────────────────────────────────────────
    And,
    Break,
    Do,
    Else,
    Elseif,
    End,
    False,
    For,
    Function,
    Goto,
    If,
    In,
    Local,
    Nil,
    Not,
    Or,
    Repeat,
    Return,
    Then,
    True,
    Until,
    While,

    // ── Attribute keywords (inside `< >`) ────────────────────────────────────
    /// `const` inside `<const>`
    AttrConst,
    /// `close` inside `<close>`
    AttrClose,

    // ── Punctuation ──────────────────────────────────────────────────────────
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Caret,       // ^
    Hash,        // #
    Ampersand,   // &
    Tilde,       // ~
    Pipe,        // |
    LtLt,        // <<
    GtGt,        // >>
    DoubleSlash, // //
    Eq,          // ==
    TildeEq,     // ~=
    LtEq,        // <=
    GtEq,        // >=
    Lt,          // <
    Gt,          // >
    Assign,      // =
    LParen,      // (
    RParen,      // )
    LBrace,      // {
    RBrace,      // }
    LBracket,    // [
    RBracket,    // ]
    DblColon,    // ::
    Semicolon,   // ;
    Colon,       // :
    Comma,       // ,
    Dot,         // .
    DotDot,      // ..
    DotDotDot,   // ...

    // ── Special ──────────────────────────────────────────────────────────────
    /// End of file.
    Eof,
}
