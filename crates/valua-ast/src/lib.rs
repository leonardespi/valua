//! Typed AST for Lua 5.5 — shared by parser, transformer, and code generator.
//!
//! Also houses [`LuaTarget`], the cross-cutting compilation-target enum used by
//! both `valua-lint` and `valua-codegen`. Placing it here avoids an illegal
//! `valua-lint → valua-codegen` dependency (see architectural note in PRD §6.2).

use valua_diagnostics::Span;

#[cfg(feature = "serde")]
use serde::Serialize;

// ── Top-level ────────────────────────────────────────────────────────────────

/// Root node: a sequence of statements followed by an optional return.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Statement>,
    pub span: Span,
}

// ── Statements ───────────────────────────────────────────────────────────────

/// Every kind of statement in Lua 5.5 (Lua 5.4 is a fully supported subset).
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub enum Statement {
    /// `local x [<attr>] = expr`
    LocalDecl(LocalDecl),
    /// `var = expr` or multi-assign
    Assign(Assign),
    /// `do … end`
    Do(Do),
    /// `while expr do … end`
    While(While),
    /// `repeat … until expr`
    Repeat(Repeat),
    /// `if … then … [elseif …] [else …] end`
    If(If),
    /// `for i = start, limit [, step] do … end`
    NumericFor(NumericFor),
    /// `for k, v in iter do … end`
    GenericFor(GenericFor),
    /// `function name(…) … end`
    FunctionDecl(FunctionDecl),
    /// `local function name(…) … end`
    LocalFunctionDecl(LocalFunctionDecl),
    /// `return [exprlist]`
    Return(Return),
    /// `break`
    Break(Span),
    /// `goto label`
    Goto(Goto),
    /// `::label::`
    Label(Label),
    /// A function-call used as a statement.
    ExprStmt(Expression),
}

/// `local varlist [<attr>] [= exprlist]`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct LocalDecl {
    pub names: Vec<LocalName>,
    pub values: Vec<Expression>,
    pub span: Span,
}

/// One name in a local declaration, with its optional attribute.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct LocalName {
    pub name: String,
    /// Lua 5.4/5.5 `<const>` or `<close>` attribute.
    pub attribute: Option<Attribute>,
    pub span: Span,
}

/// Lua 5.4/5.5 variable attributes.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribute {
    /// `<const>` — value must not change after initialisation.
    Const,
    /// `<close>` — to-be-closed variable; `__close` metamethod called on scope exit.
    Close,
}

/// Simple or multi-target assignment: `targets = values`.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct Assign {
    pub targets: Vec<Expression>,
    pub values: Vec<Expression>,
    pub span: Span,
}

/// `do … end` block.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct Do {
    pub body: Block,
    pub span: Span,
}

/// `while cond do body end`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct While {
    pub condition: Box<Expression>,
    pub body: Block,
    pub span: Span,
}

/// `repeat body until cond`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct Repeat {
    pub body: Block,
    pub condition: Box<Expression>,
    pub span: Span,
}

/// `if cond then body [elseif …]* [else …] end`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct If {
    pub condition: Box<Expression>,
    pub then_block: Block,
    pub elseif_clauses: Vec<ElseIf>,
    pub else_block: Option<Block>,
    pub span: Span,
}

/// One `elseif cond then body` branch.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct ElseIf {
    pub condition: Box<Expression>,
    pub body: Block,
    pub span: Span,
}

/// `for i = start, limit [, step] do body end`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct NumericFor {
    pub var: String,
    pub start: Box<Expression>,
    pub limit: Box<Expression>,
    pub step: Option<Box<Expression>>,
    pub body: Block,
    pub span: Span,
}

/// `for namelist in exprlist do body end`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct GenericFor {
    pub vars: Vec<String>,
    pub iterators: Vec<Expression>,
    pub body: Block,
    pub span: Span,
}

/// `function name.path:method (params) body end`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: FunctionName,
    pub func: FunctionBody,
    pub span: Span,
}

/// `local function name (params) body end`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct LocalFunctionDecl {
    pub name: String,
    pub func: FunctionBody,
    pub span: Span,
}

/// Dotted function name with optional method receiver: `a.b.c:method`.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct FunctionName {
    pub parts: Vec<String>,
    /// Present when declared as a method (`:`).
    pub method: Option<String>,
    pub span: Span,
}

/// `return [exprlist]`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct Return {
    pub values: Vec<Expression>,
    pub span: Span,
}

/// `goto label`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct Goto {
    pub label: String,
    pub span: Span,
}

/// `::label::`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct Label {
    pub name: String,
    pub span: Span,
}

// ── Function body ────────────────────────────────────────────────────────────

/// Shared representation for anonymous and named function bodies.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct FunctionBody {
    pub params: Vec<Param>,
    pub is_vararg: bool,
    pub body: Block,
    pub span: Span,
}

/// One parameter in a function signature.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub span: Span,
}

// ── Expressions ──────────────────────────────────────────────────────────────

/// All expression forms in Lua 5.5.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub enum Expression {
    Nil(Span),
    True(Span),
    False(Span),
    Integer(i64, Span),
    Float(f64, Span),
    String(String, Span),
    Vararg(Span),
    Name(String, Span),
    /// `expr.field`
    Index(Box<Expression>, String, Span),
    /// `expr[key]`
    IndexExpr(Box<Expression>, Box<Expression>, Span),
    /// Binary operation: `lhs op rhs`.
    BinOp(Box<Expression>, BinaryOp, Box<Expression>, Span),
    /// Unary operation: `op expr`.
    UnOp(UnaryOp, Box<Expression>, Span),
    /// Function call: `func(args)` or `obj:method(args)`.
    Call(Call),
    /// `function (params) body end`
    Function(FunctionBody),
    /// Table constructor: `{ [field,]* }`.
    Table(TableConstructor),
}

impl Expression {
    /// Returns the span of this expression.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn span(&self) -> Span {
        match self {
            Expression::Nil(s)
            | Expression::True(s)
            | Expression::False(s)
            | Expression::Vararg(s) => *s,
            Expression::Integer(_, s) => *s,
            Expression::Float(_, s) => *s,
            Expression::String(_, s) => *s,
            Expression::Name(_, s) => *s,
            Expression::Index(_, _, s) => *s,
            Expression::IndexExpr(_, _, s) => *s,
            Expression::BinOp(_, _, _, s) => *s,
            Expression::UnOp(_, _, s) => *s,
            Expression::Call(c) => c.span(),
            Expression::Function(f) => f.span,
            Expression::Table(t) => t.span,
        }
    }
}

// ── Operators ────────────────────────────────────────────────────────────────

/// Binary operators in Lua 5.5 (includes bitwise and integer division).
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    /// `//` integer division (5.3+).
    IDiv,
    // Comparison
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    // Logical
    And,
    Or,
    // Bitwise (5.3+)
    /// `&`
    BitwiseAnd,
    /// `|`
    BitwiseOr,
    /// `~` (binary)
    BitwiseXor,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    // String
    /// `..`
    Concat,
}

/// Unary operators in Lua 5.5.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-`
    Neg,
    /// `not`
    Not,
    /// `#`
    Len,
    /// `~` (unary bitwise NOT, 5.3+)
    BitwiseNot,
}

// ── Function calls ───────────────────────────────────────────────────────────

/// A function-call expression or method-call expression.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub enum Call {
    /// `func(args)`
    Call {
        func: Box<Expression>,
        args: Vec<Expression>,
        span: Span,
    },
    /// `obj:method(args)`
    MethodCall {
        obj: Box<Expression>,
        method: String,
        args: Vec<Expression>,
        span: Span,
    },
}

impl Call {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Call::Call { span, .. } | Call::MethodCall { span, .. } => *span,
        }
    }
}

// ── Table constructor ────────────────────────────────────────────────────────

/// `{ field, field, … }`
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct TableConstructor {
    pub fields: Vec<TableField>,
    pub span: Span,
}

/// One field in a table constructor.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone)]
pub enum TableField {
    /// `[expr] = expr`
    ExprKey {
        key: Box<Expression>,
        value: Box<Expression>,
        span: Span,
    },
    /// `name = expr`
    NameKey {
        key: String,
        value: Box<Expression>,
        span: Span,
    },
    /// `expr` (positional)
    Positional(Expression),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "TODO: verify Block can be constructed and iterated"]
    fn test_block_construction() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: verify Attribute variants are distinct"]
    fn test_attribute_variants() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: Expression::span() returns the correct span for each variant"]
    fn test_expression_span() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: BinaryOp includes all bitwise variants"]
    fn test_binary_op_bitwise_variants() {
        todo!()
    }
}

// ── Compilation target ────────────────────────────────────────────────────────

/// Target Lua runtime — controls which built-ins and polyfills are assumed available.
///
/// Lives in `valua-ast` (not `valua-codegen`) so that both `valua-lint` and
/// `valua-codegen` can depend on it without creating a cross-layer coupling.
/// `valua-codegen` re-exports this type as `pub use valua_ast::LuaTarget`.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LuaTarget {
    /// Plain Lua 5.1 (PUC reference implementation).
    #[default]
    Lua51,
    /// `LuaJIT` 2.x — has the `bit` library and some Lua 5.1 extensions.
    LuaJIT,
}
