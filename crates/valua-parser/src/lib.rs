use valua_ast::{
    Assign, Attribute, BinaryOp, Block, Call, Do, ElseIf, Expression, FunctionBody, FunctionDecl,
    FunctionName, GenericFor, Goto, If, Label, LocalDecl, LocalFunctionDecl, LocalName, NumericFor,
    Param, Repeat, Return, Statement, TableConstructor, TableField, UnaryOp, While,
};
use valua_diagnostics::Span;
use valua_lexer::{Lexer, SpannedToken, Token};

pub use error::ParseError;

mod error;

// ── Public entry point ────────────────────────────────────────────────────────

pub fn parse(source: &str) -> Result<Block, ParseError> {
    let tokens = Lexer::new(source).tokenize().map_err(ParseError::Lex)?;
    Parser::new(tokens).parse_block()
}

// ── Parser state ──────────────────────────────────────────────────────────────

pub(crate) struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub(crate) fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).map(|st| &st.token).unwrap_or(&Token::Eof)
    }

    fn peek_span(&self) -> Span {
        self.tokens.get(self.pos).map(|st| st.span).unwrap_or(Span::dummy())
    }

    fn advance(&mut self) -> &SpannedToken {
        let t = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<Span, ParseError> {
        if self.peek() == expected {
            let span = self.peek_span();
            self.advance();
            Ok(span)
        } else {
            Err(ParseError::Expected {
                expected: format!("{expected:?}"),
                found: self.peek().clone(),
                span: self.peek_span(),
            })
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        match self.peek().clone() {
            Token::Ident(name) => {
                let span = self.peek_span();
                self.advance();
                Ok((name, span))
            }
            found => Err(ParseError::Expected {
                expected: "identifier".into(),
                found,
                span: self.peek_span(),
            }),
        }
    }

    fn check(&self, tok: &Token) -> bool {
        self.peek() == tok
    }

    fn consume_if(&mut self, tok: &Token) -> Option<Span> {
        if self.peek() == tok {
            let span = self.peek_span();
            self.advance();
            Some(span)
        } else {
            None
        }
    }

    fn at_block_end(&self) -> bool {
        matches!(
            self.peek(),
            Token::End | Token::Else | Token::Elseif | Token::Until | Token::Eof
        )
    }

    // ── Block ─────────────────────────────────────────────────────────────────

    pub(crate) fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start_span = self.peek_span();
        let mut stmts = Vec::new();

        while !self.at_block_end() {
            // Skip optional semicolons
            while self.consume_if(&Token::Semicolon).is_some() {}
            if self.at_block_end() {
                break;
            }
            // `return` must be the last statement in a block
            if self.check(&Token::Return) {
                let ret = self.parse_return()?;
                stmts.push(Statement::Return(ret));
                self.consume_if(&Token::Semicolon);
                break;
            }
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
        }

        let end_span = self.peek_span();
        let span = start_span.merge(end_span);
        Ok(Block { stmts, span })
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek().clone() {
            Token::Local => {
                let span_start = self.peek_span();
                self.advance(); // consume `local`
                if self.check(&Token::Function) {
                    self.advance(); // consume `function`
                    let decl = self.parse_local_function_body(span_start)?;
                    Ok(Statement::LocalFunctionDecl(decl))
                } else {
                    let decl = self.parse_local_names(span_start)?;
                    Ok(Statement::LocalDecl(decl))
                }
            }
            Token::Function => {
                self.advance();
                let decl = self.parse_named_function()?;
                Ok(Statement::FunctionDecl(decl))
            }
            Token::Do => {
                let s = self.peek_span();
                self.advance();
                let body = self.parse_block()?;
                let e = self.expect(&Token::End)?;
                Ok(Statement::Do(Do { body, span: s.merge(e) }))
            }
            Token::While => {
                let s = self.peek_span();
                self.advance();
                let condition = Box::new(self.parse_expression(0)?);
                self.expect(&Token::Do)?;
                let body = self.parse_block()?;
                let e = self.expect(&Token::End)?;
                Ok(Statement::While(While { condition, body, span: s.merge(e) }))
            }
            Token::Repeat => {
                let s = self.peek_span();
                self.advance();
                let body = self.parse_block()?;
                self.expect(&Token::Until)?;
                let condition = Box::new(self.parse_expression(0)?);
                let e = condition.span();
                Ok(Statement::Repeat(Repeat { body, condition, span: s.merge(e) }))
            }
            Token::If => {
                let stmt = self.parse_if()?;
                Ok(Statement::If(stmt))
            }
            Token::For => {
                self.advance();
                self.parse_for_stmt()
            }
            Token::Goto => {
                let s = self.peek_span();
                self.advance();
                let (label, e) = self.expect_ident()?;
                Ok(Statement::Goto(Goto { label, span: s.merge(e) }))
            }
            Token::DblColon => {
                let s = self.peek_span();
                self.advance();
                let (name, _) = self.expect_ident()?;
                let e = self.expect(&Token::DblColon)?;
                Ok(Statement::Label(Label { name, span: s.merge(e) }))
            }
            Token::Break => {
                let s = self.peek_span();
                self.advance();
                Ok(Statement::Break(s))
            }
            _ => self.parse_assign_or_call(),
        }
    }

    fn parse_local_names(&mut self, start: Span) -> Result<LocalDecl, ParseError> {
        let mut names = Vec::new();
        loop {
            let (name, name_span) = self.expect_ident()?;
            let attribute = self.parse_attribute()?;
            names.push(LocalName { name, attribute, span: name_span });
            if self.consume_if(&Token::Comma).is_none() {
                break;
            }
        }
        let mut values = Vec::new();
        if self.consume_if(&Token::Assign).is_some() {
            values = self.parse_expr_list()?;
        }
        let end_span = values.last().map(|e| e.span()).unwrap_or(start);
        Ok(LocalDecl { names, values, span: start.merge(end_span) })
    }

    fn parse_attribute(&mut self) -> Result<Option<Attribute>, ParseError> {
        if self.consume_if(&Token::Lt).is_none() {
            return Ok(None);
        }
        let (name, span) = self.expect_ident()?;
        let attr = match name.as_str() {
            "const" => Attribute::Const,
            "close" => Attribute::Close,
            other => {
                return Err(ParseError::Expected {
                    expected: "const or close".into(),
                    found: Token::Ident(other.to_string()),
                    span,
                });
            }
        };
        self.expect(&Token::Gt)?;
        Ok(Some(attr))
    }

    fn parse_local_function_body(
        &mut self,
        start: Span,
    ) -> Result<LocalFunctionDecl, ParseError> {
        let (name, _) = self.expect_ident()?;
        let func = self.parse_function_body()?;
        let span = start.merge(func.span);
        Ok(LocalFunctionDecl { name, func, span })
    }

    fn parse_named_function(&mut self) -> Result<FunctionDecl, ParseError> {
        let name = self.parse_function_name()?;
        let func = self.parse_function_body()?;
        let span = name.span.merge(func.span);
        Ok(FunctionDecl { name, func, span })
    }

    fn parse_function_name(&mut self) -> Result<FunctionName, ParseError> {
        let (first, start) = self.expect_ident()?;
        let mut parts = vec![first];
        let mut end = start;
        while self.consume_if(&Token::Dot).is_some() {
            let (part, s) = self.expect_ident()?;
            end = s;
            parts.push(part);
        }
        let method = if self.consume_if(&Token::Colon).is_some() {
            let (m, s) = self.expect_ident()?;
            end = s;
            Some(m)
        } else {
            None
        };
        Ok(FunctionName { parts, method, span: start.merge(end) })
    }

    fn parse_function_body(&mut self) -> Result<FunctionBody, ParseError> {
        let s = self.expect(&Token::LParen)?;
        let (params, is_vararg) = self.parse_param_list()?;
        self.expect(&Token::RParen)?;
        let body = self.parse_block()?;
        let e = self.expect(&Token::End)?;
        Ok(FunctionBody { params, is_vararg, body, span: s.merge(e) })
    }

    fn parse_param_list(&mut self) -> Result<(Vec<Param>, bool), ParseError> {
        let mut params = Vec::new();
        let mut is_vararg = false;
        if self.check(&Token::RParen) {
            return Ok((params, is_vararg));
        }
        loop {
            if self.check(&Token::DotDotDot) {
                let s = self.peek_span();
                self.advance();
                is_vararg = true;
                let _ = s;
                break;
            }
            let (name, span) = self.expect_ident()?;
            params.push(Param { name, span });
            if self.consume_if(&Token::Comma).is_none() {
                break;
            }
        }
        Ok((params, is_vararg))
    }

    fn parse_if(&mut self) -> Result<If, ParseError> {
        let s = self.peek_span();
        self.expect(&Token::If)?;
        let condition = Box::new(self.parse_expression(0)?);
        self.expect(&Token::Then)?;
        let then_block = self.parse_block()?;

        let mut elseif_clauses = Vec::new();
        let mut else_block = None;
        loop {
            if self.check(&Token::Elseif) {
                let es = self.peek_span();
                self.advance();
                let cond = Box::new(self.parse_expression(0)?);
                self.expect(&Token::Then)?;
                let body = self.parse_block()?;
                let span = es.merge(body.span);
                elseif_clauses.push(ElseIf { condition: cond, body, span });
            } else if self.consume_if(&Token::Else).is_some() {
                else_block = Some(self.parse_block()?);
                break;
            } else {
                break;
            }
        }
        let e = self.expect(&Token::End)?;
        Ok(If { condition, then_block, elseif_clauses, else_block, span: s.merge(e) })
    }

    fn parse_for_stmt(&mut self) -> Result<Statement, ParseError> {
        let (first_name, start) = self.expect_ident()?;
        if self.check(&Token::Assign) {
            // numeric for: name = start, limit [, step]
            self.advance();
            let init = self.parse_expression(0)?;
            self.expect(&Token::Comma)?;
            let limit = self.parse_expression(0)?;
            let step = if self.consume_if(&Token::Comma).is_some() {
                Some(Box::new(self.parse_expression(0)?))
            } else {
                None
            };
            self.expect(&Token::Do)?;
            let body = self.parse_block()?;
            let e = self.expect(&Token::End)?;
            Ok(Statement::NumericFor(NumericFor {
                var: first_name,
                start: Box::new(init),
                limit: Box::new(limit),
                step,
                body,
                span: start.merge(e),
            }))
        } else {
            // generic for: namelist in exprlist do body end
            let mut vars = vec![first_name];
            while self.consume_if(&Token::Comma).is_some() {
                let (name, _) = self.expect_ident()?;
                vars.push(name);
            }
            self.expect(&Token::In)?;
            let iterators = self.parse_expr_list()?;
            self.expect(&Token::Do)?;
            let body = self.parse_block()?;
            let e = self.expect(&Token::End)?;
            Ok(Statement::GenericFor(GenericFor {
                vars,
                iterators,
                body,
                span: start.merge(e),
            }))
        }
    }

    fn parse_return(&mut self) -> Result<Return, ParseError> {
        let s = self.peek_span();
        self.expect(&Token::Return)?;
        let values = if self.at_block_end() || self.check(&Token::Semicolon) {
            Vec::new()
        } else {
            self.parse_expr_list()?
        };
        let end = values.last().map(|e| e.span()).unwrap_or(s);
        Ok(Return { values, span: s.merge(end) })
    }

    fn parse_assign_or_call(&mut self) -> Result<Statement, ParseError> {
        let expr = self.parse_suffixed_expression()?;

        if self.check(&Token::Assign) || self.check(&Token::Comma) {
            // Assignment: collect more targets then values
            let mut targets = vec![expr];
            while self.consume_if(&Token::Comma).is_some() {
                targets.push(self.parse_suffixed_expression()?);
            }
            self.expect(&Token::Assign)?;
            let values = self.parse_expr_list()?;
            let span_start = targets[0].span();
            let span_end = values.last().map(|e| e.span()).unwrap_or(span_start);
            Ok(Statement::Assign(Assign {
                targets,
                values,
                span: span_start.merge(span_end),
            }))
        } else {
            // Must be a call statement
            match &expr {
                Expression::Call(_) => Ok(Statement::ExprStmt(expr)),
                _ => Err(ParseError::Unexpected {
                    found: self.peek().clone(),
                    span: self.peek_span(),
                }),
            }
        }
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn parse_expr_list(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut exprs = vec![self.parse_expression(0)?];
        while self.consume_if(&Token::Comma).is_some() {
            exprs.push(self.parse_expression(0)?);
        }
        Ok(exprs)
    }

    /// Pratt/precedence-climbing expression parser.
    fn parse_expression(&mut self, min_prec: u8) -> Result<Expression, ParseError> {
        let mut lhs = self.parse_unary_expression()?;

        loop {
            let Some((op, (left_bp, right_bp))) = self.peek_binary_op() else {
                break;
            };
            if left_bp < min_prec {
                break;
            }
            self.advance(); // consume operator token
            let rhs = self.parse_expression(right_bp)?;
            let span = lhs.span().merge(rhs.span());
            lhs = Expression::BinOp(Box::new(lhs), op, Box::new(rhs), span);
        }

        Ok(lhs)
    }

    fn peek_binary_op(&self) -> Option<(BinaryOp, (u8, u8))> {
        let op = match self.peek() {
            Token::Or => BinaryOp::Or,
            Token::And => BinaryOp::And,
            Token::Lt => BinaryOp::Lt,
            Token::Gt => BinaryOp::Gt,
            Token::LtEq => BinaryOp::Le,
            Token::GtEq => BinaryOp::Ge,
            Token::Eq => BinaryOp::Eq,
            Token::TildeEq => BinaryOp::Ne,
            Token::Pipe => BinaryOp::BitwiseOr,
            Token::Tilde => BinaryOp::BitwiseXor,
            Token::Ampersand => BinaryOp::BitwiseAnd,
            Token::LtLt => BinaryOp::Shl,
            Token::GtGt => BinaryOp::Shr,
            Token::DotDot => BinaryOp::Concat,
            Token::Plus => BinaryOp::Add,
            Token::Minus => BinaryOp::Sub,
            Token::Star => BinaryOp::Mul,
            Token::Slash => BinaryOp::Div,
            Token::DoubleSlash => BinaryOp::IDiv,
            Token::Percent => BinaryOp::Mod,
            Token::Caret => BinaryOp::Pow,
            _ => return None,
        };
        Some((op, binary_prec(op)))
    }

    fn parse_unary_expression(&mut self) -> Result<Expression, ParseError> {
        let s = self.peek_span();
        let op = match self.peek() {
            Token::Minus => Some(UnaryOp::Neg),
            Token::Not => Some(UnaryOp::Not),
            Token::Hash => Some(UnaryOp::Len),
            Token::Tilde => Some(UnaryOp::BitwiseNot),
            _ => None,
        };
        if let Some(unop) = op {
            self.advance();
            // Unary operators bind tighter than all binary ops except `^` (prec 23).
            let operand = self.parse_expression(UNARY_PREC)?;
            let span = s.merge(operand.span());
            Ok(Expression::UnOp(unop, Box::new(operand), span))
        } else {
            self.parse_suffixed_expression()
        }
    }

    fn parse_suffixed_expression(&mut self) -> Result<Expression, ParseError> {
        let mut base = self.parse_primary_expression()?;
        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    let (field, s) = self.expect_ident()?;
                    let span = base.span().merge(s);
                    base = Expression::Index(Box::new(base), field, span);
                }
                Token::LBracket => {
                    let s = self.peek_span();
                    self.advance();
                    let key = self.parse_expression(0)?;
                    let e = self.expect(&Token::RBracket)?;
                    let span = s.merge(e);
                    base = Expression::IndexExpr(Box::new(base), Box::new(key), span);
                }
                Token::Colon => {
                    self.advance();
                    let (method, _) = self.expect_ident()?;
                    let (args, e) = self.parse_call_args()?;
                    let span = base.span().merge(e);
                    base = Expression::Call(Call::MethodCall {
                        obj: Box::new(base),
                        method,
                        args,
                        span,
                    });
                }
                Token::LParen | Token::StringLit(_) | Token::LBrace => {
                    let (args, e) = self.parse_call_args()?;
                    let span = base.span().merge(e);
                    base = Expression::Call(Call::Call {
                        func: Box::new(base),
                        args,
                        span,
                    });
                }
                _ => break,
            }
        }
        Ok(base)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, ParseError> {
        let s = self.peek_span();
        match self.peek().clone() {
            Token::Nil => {
                self.advance();
                Ok(Expression::Nil(s))
            }
            Token::True => {
                self.advance();
                Ok(Expression::True(s))
            }
            Token::False => {
                self.advance();
                Ok(Expression::False(s))
            }
            Token::Integer(n) => {
                self.advance();
                Ok(Expression::Integer(n, s))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expression::Float(f, s))
            }
            Token::StringLit(text) => {
                self.advance();
                Ok(Expression::String(text, s))
            }
            Token::DotDotDot => {
                self.advance();
                Ok(Expression::Vararg(s))
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Expression::Name(name, s))
            }
            Token::LParen => {
                self.advance();
                let inner = self.parse_expression(0)?;
                let e = self.expect(&Token::RParen)?;
                Ok(adjust_span(inner, s.merge(e)))
            }
            Token::LBrace => {
                let tbl = self.parse_table_constructor()?;
                Ok(Expression::Table(tbl))
            }
            Token::Function => {
                self.advance();
                let body = self.parse_function_body()?;
                Ok(Expression::Function(body))
            }
            found => Err(ParseError::Unexpected { found, span: s }),
        }
    }

    fn parse_call_args(&mut self) -> Result<(Vec<Expression>, Span), ParseError> {
        match self.peek().clone() {
            Token::LParen => {
                let s = self.peek_span();
                self.advance();
                let args = if self.check(&Token::RParen) {
                    Vec::new()
                } else {
                    self.parse_expr_list()?
                };
                let e = self.expect(&Token::RParen)?;
                Ok((args, s.merge(e)))
            }
            Token::StringLit(text) => {
                let s = self.peek_span();
                self.advance();
                Ok((vec![Expression::String(text, s)], s))
            }
            Token::LBrace => {
                let tbl = self.parse_table_constructor()?;
                let span = tbl.span;
                Ok((vec![Expression::Table(tbl)], span))
            }
            found => Err(ParseError::Expected {
                expected: "call arguments".into(),
                found,
                span: self.peek_span(),
            }),
        }
    }

    fn parse_table_constructor(&mut self) -> Result<TableConstructor, ParseError> {
        let s = self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) {
            fields.push(self.parse_table_field()?);
            if self.consume_if(&Token::Comma).is_none()
                && self.consume_if(&Token::Semicolon).is_none()
            {
                break;
            }
        }
        let e = self.expect(&Token::RBrace)?;
        Ok(TableConstructor { fields, span: s.merge(e) })
    }

    fn parse_table_field(&mut self) -> Result<TableField, ParseError> {
        match self.peek().clone() {
            Token::LBracket => {
                let s = self.peek_span();
                self.advance();
                let key = self.parse_expression(0)?;
                self.expect(&Token::RBracket)?;
                self.expect(&Token::Assign)?;
                let value = Box::new(self.parse_expression(0)?);
                let span = s.merge(value.span());
                Ok(TableField::ExprKey { key: Box::new(key), value, span })
            }
            Token::Ident(name) if self.is_name_assign_field() => {
                let s = self.peek_span();
                self.advance(); // name
                self.advance(); // `=`
                let value = Box::new(self.parse_expression(0)?);
                let span = s.merge(value.span());
                Ok(TableField::NameKey { key: name, value, span })
            }
            _ => Ok(TableField::Positional(self.parse_expression(0)?)),
        }
    }

    /// Check if upcoming tokens are `Name =` (table field with name key).
    fn is_name_assign_field(&self) -> bool {
        matches!(self.tokens.get(self.pos), Some(st) if matches!(st.token, Token::Ident(_)))
            && matches!(
                self.tokens.get(self.pos + 1),
                Some(st) if st.token == Token::Assign
            )
    }
}

// ── Operator precedence table ─────────────────────────────────────────────────

/// Unary operators bind at this minimum-prec level.
/// Set between `*/%` (19–20) and `^` (23–24) so unary is tighter than `*` but looser than `^`.
const UNARY_PREC: u8 = 21;

/// Returns `(left_bp, right_bp)` for a binary operator.
/// Left-assoc: `(n, n+1)`. Right-assoc: `(n+1, n)`.
fn binary_prec(op: BinaryOp) -> (u8, u8) {
    match op {
        BinaryOp::Or => (1, 2),
        BinaryOp::And => (3, 4),
        BinaryOp::Lt
        | BinaryOp::Gt
        | BinaryOp::Le
        | BinaryOp::Ge
        | BinaryOp::Eq
        | BinaryOp::Ne => (5, 6),
        BinaryOp::BitwiseOr => (7, 8),
        BinaryOp::BitwiseXor => (9, 10),
        BinaryOp::BitwiseAnd => (11, 12),
        BinaryOp::Shl | BinaryOp::Shr => (13, 14),
        BinaryOp::Concat => (16, 15), // right-associative
        BinaryOp::Add | BinaryOp::Sub => (17, 18),
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::IDiv | BinaryOp::Mod => (19, 20),
        BinaryOp::Pow => (24, 23), // right-associative
    }
}

fn adjust_span(expr: Expression, span: Span) -> Expression {
    match expr {
        // Leaf types: reconstruct with new span.
        Expression::Nil(_) => Expression::Nil(span),
        Expression::True(_) => Expression::True(span),
        Expression::False(_) => Expression::False(span),
        Expression::Integer(v, _) => Expression::Integer(v, span),
        Expression::Float(v, _) => Expression::Float(v, span),
        Expression::String(v, _) => Expression::String(v, span),
        Expression::Vararg(_) => Expression::Vararg(span),
        Expression::Name(v, _) => Expression::Name(v, span),
        // Tuple-span variants: deconstruct, drop old span, reconstruct.
        Expression::BinOp(lhs, op, rhs, _) => Expression::BinOp(lhs, op, rhs, span),
        Expression::UnOp(op, operand, _) => Expression::UnOp(op, operand, span),
        Expression::Index(base, field, _) => Expression::Index(base, field, span),
        Expression::IndexExpr(base, key, _) => Expression::IndexExpr(base, key, span),
        // Call: update the named span field inside the Call enum.
        Expression::Call(call) => Expression::Call(match call {
            Call::Call { func, args, .. } => Call::Call { func, args, span },
            Call::MethodCall { obj, method, args, .. } => {
                Call::MethodCall { obj, method, args, span }
            }
        }),
        // Named-span structs: mutate the span field directly.
        Expression::Function(mut body) => {
            body.span = span;
            Expression::Function(body)
        }
        Expression::Table(mut tbl) => {
            tbl.span = span;
            Expression::Table(tbl)
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Block {
        parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"))
    }

    #[test]
    fn test_parse_empty_block() {
        let block = parse_ok("");
        assert!(block.stmts.is_empty());
    }

    #[test]
    fn test_parse_local_decl() {
        let block = parse_ok("local x = 1");
        assert!(matches!(block.stmts[0], Statement::LocalDecl(_)));
        if let Statement::LocalDecl(d) = &block.stmts[0] {
            assert_eq!(d.names[0].name, "x");
            assert!(matches!(d.values[0], Expression::Integer(1, _)));
        }
    }

    #[test]
    fn test_parse_const_attribute() {
        let block = parse_ok("local x <const> = 1");
        if let Statement::LocalDecl(d) = &block.stmts[0] {
            assert_eq!(d.names[0].attribute, Some(Attribute::Const));
        }
    }

    #[test]
    fn test_parse_close_attribute() {
        let block = parse_ok(r#"local f <close> = io.open("f.txt", "r")"#);
        if let Statement::LocalDecl(d) = &block.stmts[0] {
            assert_eq!(d.names[0].attribute, Some(Attribute::Close));
        }
    }

    #[test]
    fn test_parse_bitwise_expr() {
        let block = parse_ok("local x = a & b");
        if let Statement::LocalDecl(d) = &block.stmts[0] {
            assert!(matches!(
                d.values[0],
                Expression::BinOp(_, BinaryOp::BitwiseAnd, _, _)
            ));
        }
    }

    #[test]
    fn test_parse_all_fixture_inputs() {
        let fixtures = [
            ("bitwise_and", include_str!("../../../tests/fixtures/bitwise_and/input.lua")),
            ("bitwise_or", include_str!("../../../tests/fixtures/bitwise_or/input.lua")),
            ("bitwise_xor", include_str!("../../../tests/fixtures/bitwise_xor/input.lua")),
            ("bitwise_not", include_str!("../../../tests/fixtures/bitwise_not/input.lua")),
            ("shift_left", include_str!("../../../tests/fixtures/shift_left/input.lua")),
            ("shift_right", include_str!("../../../tests/fixtures/shift_right/input.lua")),
            ("integer_division", include_str!("../../../tests/fixtures/integer_division/input.lua")),
            ("const_attribute", include_str!("../../../tests/fixtures/const_attribute/input.lua")),
            ("close_simple", include_str!("../../../tests/fixtures/close_attribute_simple/input.lua")),
            ("close_error", include_str!("../../../tests/fixtures/close_attribute_error_path/input.lua")),
            ("E0101", include_str!("../../../tests/fixtures/errors/E0101_math_type/input.lua")),
            ("E0102", include_str!("../../../tests/fixtures/errors/E0102_integer_overflow/input.lua")),
            ("E0301", include_str!("../../../tests/fixtures/errors/E0301_const_mutation/input.lua")),
        ];
        for (name, src) in fixtures {
            parse(src).unwrap_or_else(|e| panic!("fixture {name} parse failed: {e}"));
        }
    }

    #[test]
    fn test_paren_adjusts_span_on_binop() {
        // (a + b) — the BinOp node must adopt the outer () span.
        let block = parse_ok("local x = (a + b)");
        if let Statement::LocalDecl(d) = &block.stmts[0] {
            let expr = &d.values[0];
            // Inner span of `a + b` starts at `a`. Outer span starts at `(`.
            // With adjust_span fixed, expr.span().start points at `(`.
            assert!(
                matches!(expr, Expression::BinOp(_, BinaryOp::Add, _, _)),
                "expected BinOp"
            );
            // `local x = ` is 10 bytes; `(` is at offset 10.
            assert_eq!(expr.span().start, 10, "BinOp span should start at `(`");
            // closing `)` is at offset 16; end is exclusive so 17.
            assert_eq!(expr.span().end, 17, "BinOp span should end after `)`");
        } else {
            panic!("expected LocalDecl");
        }
    }

    #[test]
    fn test_paren_adjusts_span_on_unop() {
        let block = parse_ok("local x = (#t)");
        if let Statement::LocalDecl(d) = &block.stmts[0] {
            let expr = &d.values[0];
            assert!(matches!(expr, Expression::UnOp(UnaryOp::Len, _, _)), "expected UnOp Len");
            assert_eq!(expr.span().start, 10, "UnOp span should start at `(`");
        } else {
            panic!("expected LocalDecl");
        }
    }

    #[test]
    fn test_paren_adjusts_span_on_call() {
        let block = parse_ok("local x = (f())");
        if let Statement::LocalDecl(d) = &block.stmts[0] {
            let expr = &d.values[0];
            assert!(matches!(expr, Expression::Call(_)), "expected Call");
            assert_eq!(expr.span().start, 10, "Call span should start at outer `(`");
        } else {
            panic!("expected LocalDecl");
        }
    }

    #[test]
    fn test_parse_e0401_does_not_panic() {
        // Unknown attribute returns Err, not panic.
        let src = include_str!("../../../tests/fixtures/errors/E0401_post55_feature/input.lua");
        let _ = parse(src); // ok if Err, not ok if panic
    }
}
