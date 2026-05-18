//! Recursive-descent parser that turns a token stream into a Lua 5.4 `Block`.

use valua_ast::{
    Attribute, Block, Do, Expression, FunctionBody, FunctionDecl, FunctionName, GenericFor, Goto,
    If, Label, LocalDecl, LocalFunctionDecl, NumericFor, Repeat, Return, Statement,
    TableConstructor, TableField, While,
};
use valua_diagnostics::Span;
use valua_lexer::{Lexer, SpannedToken, Token};

pub use error::ParseError;

mod error;

/// Parses Lua 5.4 source text into an AST `Block`.
///
/// Lexes then parses in a single pass; returns the first error encountered.
pub fn parse(source: &str) -> Result<Block, ParseError> {
    let tokens = Lexer::new(source).tokenize().map_err(ParseError::Lex)?;
    Parser::new(tokens).parse_block()
}

/// Stateful recursive-descent parser.
pub(crate) struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub(crate) fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    // ── Entry point ──────────────────────────────────────────────────────────

    /// Parse a full block (top-level or function body).
    pub(crate) fn parse_block(&mut self) -> Result<Block, ParseError> {
        // TODO: loop collecting statements until at_block_end(), then build Block
        let _ = self.at_block_end();
        let _ = self.peek_span();
        let _ = self.parse_statement();
        todo!("collect statements into Block until at_block_end()")
    }

    // ── Statements ───────────────────────────────────────────────────────────

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        // TODO: dispatch on peek() to the correct sub-parser
        let _ = self.parse_local_decl();
        let _ = self.parse_assign_or_call();
        let _ = self.parse_do();
        let _ = self.parse_while();
        let _ = self.parse_repeat();
        let _ = self.parse_if();
        let _ = self.parse_numeric_for();
        let _ = self.parse_generic_for();
        let _ = self.parse_function_decl();
        let _ = self.parse_local_function_decl();
        let _ = self.parse_return();
        let _ = self.parse_goto();
        let _ = self.parse_label();
        todo!("dispatch on peek() token kind")
    }

    fn parse_local_decl(&mut self) -> Result<LocalDecl, ParseError> {
        // TODO: `local` Name {`,` Name} [<attr>] [`=` exprlist]
        let _ = self.parse_attribute();
        let _ = self.parse_expression();
        todo!("parse local declaration with optional attribute list")
    }

    fn parse_attribute(&mut self) -> Result<Option<Attribute>, ParseError> {
        // TODO: if peek is `<`, consume `const` or `close` then `>`
        todo!("parse optional <const> or <close> attribute")
    }

    fn parse_assign_or_call(&mut self) -> Result<Statement, ParseError> {
        // TODO: parse prefixexp; assignment if `=` follows, call-stmt otherwise
        let _ = self.parse_expression();
        todo!("disambiguate assignment from call statement")
    }

    fn parse_do(&mut self) -> Result<Do, ParseError> {
        // TODO: consume `do`, parse block, consume `end`
        let _ = self.expect(&Token::Do);
        let _ = self.parse_block();
        todo!("parse do…end block")
    }

    fn parse_while(&mut self) -> Result<While, ParseError> {
        // TODO: consume `while`, parse condition, `do`, body, `end`
        let _ = self.parse_expression();
        let _ = self.parse_block();
        todo!("parse while loop")
    }

    fn parse_repeat(&mut self) -> Result<Repeat, ParseError> {
        // TODO: consume `repeat`, parse body, `until`, condition
        let _ = self.parse_block();
        let _ = self.parse_expression();
        todo!("parse repeat…until loop")
    }

    fn parse_if(&mut self) -> Result<If, ParseError> {
        // TODO: parse if/elseif/else/end chain
        let _ = self.parse_expression();
        let _ = self.parse_block();
        todo!("parse if…then…[elseif…then…]*[else…]end")
    }

    fn parse_numeric_for(&mut self) -> Result<NumericFor, ParseError> {
        // TODO: `for` name `=` start `,` limit [`,` step] `do` body `end`
        let _ = self.parse_expression();
        let _ = self.parse_block();
        todo!("parse numeric for loop")
    }

    fn parse_generic_for(&mut self) -> Result<GenericFor, ParseError> {
        // TODO: `for` namelist `in` exprlist `do` body `end`
        let _ = self.parse_expression();
        let _ = self.parse_block();
        todo!("parse generic for loop")
    }

    fn parse_function_decl(&mut self) -> Result<FunctionDecl, ParseError> {
        // TODO: `function` funcname funcbody
        let _ = self.parse_function_name();
        let _ = self.parse_function_body();
        todo!("parse named function declaration")
    }

    fn parse_local_function_decl(&mut self) -> Result<LocalFunctionDecl, ParseError> {
        // TODO: `local` `function` Name funcbody
        let _ = self.parse_function_body();
        todo!("parse local function declaration")
    }

    fn parse_return(&mut self) -> Result<Return, ParseError> {
        // TODO: `return` [exprlist] [`;`]
        let _ = self.parse_expression();
        todo!("parse return statement")
    }

    fn parse_goto(&mut self) -> Result<Goto, ParseError> {
        // TODO: `goto` Name
        todo!("parse goto statement")
    }

    fn parse_label(&mut self) -> Result<Label, ParseError> {
        // TODO: `::` Name `::`
        todo!("parse label declaration")
    }

    // ── Function body ─────────────────────────────────────────────────────────

    fn parse_function_body(&mut self) -> Result<FunctionBody, ParseError> {
        // TODO: `(` [parlist] `)` block `end`
        let _ = self.parse_block();
        todo!("parse function parameter list and body")
    }

    fn parse_function_name(&mut self) -> Result<FunctionName, ParseError> {
        // TODO: Name {`.` Name} [`:` Name]
        todo!("parse dotted function name with optional method receiver")
    }

    // ── Expressions ──────────────────────────────────────────────────────────

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        // TODO: entry for precedence-climbing parser
        let lhs = self.parse_unary_expression()?;
        let _ = self.parse_expression_with_precedence(0);
        let _ = self.parse_suffix_expression(lhs.clone());
        todo!("complete precedence-climbing expression parser")
    }

    fn parse_expression_with_precedence(
        &mut self,
        _min_prec: u8,
    ) -> Result<Expression, ParseError> {
        // TODO: precedence-climbing loop for binary operators
        todo!("precedence-climbing binary operator loop")
    }

    fn parse_unary_expression(&mut self) -> Result<Expression, ParseError> {
        // TODO: handle `-`, `not`, `#`, `~` prefixes
        self.parse_primary_expression()
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, ParseError> {
        // TODO: literals, names, `(expr)`, table constructors, function defs
        let _ = self.parse_table_constructor();
        let _ = self.parse_function_body();
        let _ = self.parse_call_args();
        todo!("parse primary (leaf) expression")
    }

    fn parse_suffix_expression(&mut self, _base: Expression) -> Result<Expression, ParseError> {
        // TODO: handle `.field`, `[key]`, `(args)`, `:method(args)` chains
        todo!("parse suffix chains (index / call)")
    }

    fn parse_table_constructor(&mut self) -> Result<TableConstructor, ParseError> {
        // TODO: `{` fieldlist `}`
        let _ = self.parse_table_field();
        todo!("parse table constructor")
    }

    fn parse_table_field(&mut self) -> Result<TableField, ParseError> {
        // TODO: `[expr]=expr` | `Name=expr` | `expr`
        let _ = self.parse_expression();
        todo!("parse table field")
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expression>, ParseError> {
        // TODO: `(` exprlist `)` | string literal | table constructor
        let _ = self.parse_expression();
        todo!("parse function call argument list")
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).map(|st| &st.token).unwrap_or(&Token::Eof)
    }

    fn peek_span(&self) -> Span {
        self.tokens.get(self.pos).map(|st| st.span).unwrap_or(Span::dummy())
    }

    fn advance(&mut self) -> &SpannedToken {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t
    }

    fn expect(&mut self, _expected: &Token) -> Result<Span, ParseError> {
        // TODO: verify peek() matches `_expected`, then advance; else ParseError::Expected
        let st = self.advance();
        let span = st.span;
        todo!("verify token matches expected before returning span {span:?}")
    }

    fn at_block_end(&self) -> bool {
        let _ = self.peek();
        matches!(
            self.peek(),
            Token::End | Token::Else | Token::Elseif | Token::Until | Token::Eof
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "TODO: test_parse_empty_block — '' produces empty Block"]
    fn test_parse_empty_block() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_parse_local_decl — 'local x = 1' produces LocalDecl"]
    fn test_parse_local_decl() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_parse_const_attribute — 'local x <const> = 1' sets Attribute::Const"]
    fn test_parse_const_attribute() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_parse_close_attribute — 'local f <close> = io.open(...)' sets Attribute::Close"]
    fn test_parse_close_attribute() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_parse_bitwise_expr — 'a & b' parses to BinaryOp::BitwiseAnd"]
    fn test_parse_bitwise_expr() {
        todo!()
    }
}
