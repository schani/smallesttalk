use crate::compiler::{
    ast::{
        Block, CascadeMessage, Expression, Literal, MessageKind, MethodDef, MethodPattern,
        PseudoVar, Statement,
    },
    scanner::{ScanError, scan},
    token::{Span, Token, TokenKind},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ParseError {}

impl From<ScanError> for ParseError {
    fn from(value: ScanError) -> Self {
        Self {
            message: value.message,
            span: value.span,
        }
    }
}

pub fn parse_method(source: &str) -> Result<MethodDef, ParseError> {
    let tokens = scan(source)?;
    Parser::new(tokens).parse_method()
}

pub fn parse_expression(source: &str) -> Result<Expression, ParseError> {
    let tokens = scan(source)?;
    Parser::new(tokens).parse_expression_then_end()
}

pub fn parse_doit(source: &str) -> Result<MethodDef, ParseError> {
    let tokens = scan(source)?;
    Parser::new(tokens).parse_doit()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_method(&mut self) -> Result<MethodDef, ParseError> {
        let start = self.current().span.start;
        let pattern = self.parse_method_pattern()?;
        let temps = self.parse_temporaries()?;
        let statements = self.parse_statements(TokenKindMatcher::End)?;
        self.expect_end()?;
        Ok(MethodDef {
            span: Span::new(start, self.current().span.end),
            pattern,
            temps,
            statements,
        })
    }

    fn parse_doit(&mut self) -> Result<MethodDef, ParseError> {
        let start = self.current().span.start;
        let temps = self.parse_temporaries()?;
        let statements = self.parse_statements(TokenKindMatcher::End)?;
        self.expect_end()?;
        Ok(MethodDef {
            pattern: MethodPattern {
                selector: "DoIt".to_string(),
                arguments: Vec::new(),
                kind: MessageKind::Unary,
                span: Span::new(start, start),
            },
            temps,
            statements,
            span: Span::new(start, self.current().span.end),
        })
    }

    fn parse_expression_then_end(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_expression()?;
        self.expect_end()?;
        Ok(expr)
    }

    fn parse_method_pattern(&mut self) -> Result<MethodPattern, ParseError> {
        let start = self.current().span.start;
        let (selector, arguments, kind) = match &self.current().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                (name, Vec::new(), MessageKind::Unary)
            }
            TokenKind::BinarySelector(selector) => {
                let selector = selector.clone();
                self.advance();
                let argument = self.expect_identifier()?;
                (selector, vec![argument], MessageKind::Binary)
            }
            TokenKind::Keyword(_) => {
                let mut selector = String::new();
                let mut arguments = Vec::new();
                while let TokenKind::Keyword(part) = &self.current().kind {
                    selector.push_str(part);
                    self.advance();
                    arguments.push(self.expect_identifier()?);
                }
                (selector, arguments, MessageKind::Keyword)
            }
            _ => {
                return Err(self.error_here("expected method pattern"));
            }
        };
        Ok(MethodPattern {
            span: Span::new(start, self.previous().span.end),
            selector,
            arguments,
            kind,
        })
    }

    fn parse_temporaries(&mut self) -> Result<Vec<String>, ParseError> {
        let mut temps = Vec::new();
        if !matches!(self.current().kind, TokenKind::Bar) {
            return Ok(temps);
        }
        self.advance();
        while !matches!(self.current().kind, TokenKind::Bar | TokenKind::End) {
            temps.push(self.expect_identifier()?);
        }
        self.expect(TokenKindMatcher::Bar, "expected closing | for temporaries")?;
        Ok(temps)
    }

    fn parse_statements(
        &mut self,
        terminator: TokenKindMatcher,
    ) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();
        while !terminator.matches(&self.current().kind)
            && !matches!(self.current().kind, TokenKind::End)
        {
            let statement = self.parse_statement()?;
            statements.push(statement);
            if matches!(self.current().kind, TokenKind::Period) {
                self.advance();
                while matches!(self.current().kind, TokenKind::Period) {
                    self.advance();
                }
                continue;
            }
            if terminator.matches(&self.current().kind)
                || matches!(self.current().kind, TokenKind::End)
            {
                break;
            }
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if matches!(self.current().kind, TokenKind::Return) {
            let start = self.current().span.start;
            self.advance();
            let value = self.parse_expression()?;
            return Ok(Statement::Return {
                span: Span::new(start, self.span_of_expr(&value).end),
                value,
            });
        }

        if let TokenKind::Identifier(name) = &self.current().kind {
            if matches!(self.peek().kind, TokenKind::Assign) {
                let start = self.current().span.start;
                let name = name.clone();
                self.advance();
                self.advance();
                let value = self.parse_expression()?;
                return Ok(Statement::Assignment {
                    span: Span::new(start, self.span_of_expr(&value).end),
                    name,
                    value,
                });
            }
        }

        Ok(Statement::Expression(self.parse_expression()?))
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        let head = self.parse_keyword_expression()?;
        if !matches!(self.current().kind, TokenKind::Semicolon) {
            return Ok(head);
        }
        let start = self.span_of_expr(&head).start;
        let mut messages = Vec::new();
        while matches!(self.current().kind, TokenKind::Semicolon) {
            self.advance();
            messages.push(self.parse_cascade_message()?);
        }
        let end = messages
            .last()
            .map(|message| message.span.end)
            .unwrap_or_else(|| self.span_of_expr(&head).end);
        Ok(Expression::Cascade {
            head: Box::new(head),
            messages,
            span: Span::new(start, end),
        })
    }

    fn parse_cascade_message(&mut self) -> Result<CascadeMessage, ParseError> {
        let start = self.current().span.start;
        match &self.current().kind {
            TokenKind::Identifier(name) => {
                let selector = name.clone();
                self.advance();
                Ok(CascadeMessage {
                    selector,
                    arguments: Vec::new(),
                    kind: MessageKind::Unary,
                    span: Span::new(start, self.previous().span.end),
                })
            }
            TokenKind::BinarySelector(selector) => {
                let selector = selector.clone();
                self.advance();
                let argument = self.parse_unary_expression()?;
                let end = self.span_of_expr(&argument).end;
                Ok(CascadeMessage {
                    selector,
                    arguments: vec![argument],
                    kind: MessageKind::Binary,
                    span: Span::new(start, end),
                })
            }
            TokenKind::Keyword(_) => {
                let mut selector = String::new();
                let mut arguments = Vec::new();
                while let TokenKind::Keyword(part) = &self.current().kind {
                    selector.push_str(part);
                    self.advance();
                    arguments.push(self.parse_binary_expression()?);
                }
                Ok(CascadeMessage {
                    selector,
                    arguments,
                    kind: MessageKind::Keyword,
                    span: Span::new(start, self.previous().span.end),
                })
            }
            _ => Err(self.error_here("expected cascade message")),
        }
    }

    fn parse_keyword_expression(&mut self) -> Result<Expression, ParseError> {
        let mut receiver = self.parse_binary_expression()?;
        if !matches!(self.current().kind, TokenKind::Keyword(_)) {
            return Ok(receiver);
        }

        let start = self.span_of_expr(&receiver).start;
        let mut selector = String::new();
        let mut arguments = Vec::new();
        while let TokenKind::Keyword(part) = &self.current().kind {
            selector.push_str(part);
            self.advance();
            arguments.push(self.parse_binary_expression()?);
        }
        let end = arguments
            .last()
            .map(|expr| self.span_of_expr(expr).end)
            .unwrap_or(self.previous().span.end);
        receiver = Expression::Send {
            receiver: Box::new(receiver),
            selector,
            arguments,
            kind: MessageKind::Keyword,
            span: Span::new(start, end),
        };
        Ok(receiver)
    }

    fn parse_binary_expression(&mut self) -> Result<Expression, ParseError> {
        let mut receiver = self.parse_unary_expression()?;
        while let TokenKind::BinarySelector(selector) = &self.current().kind {
            let start = self.span_of_expr(&receiver).start;
            let selector = selector.clone();
            self.advance();
            let argument = self.parse_unary_expression()?;
            let end = self.span_of_expr(&argument).end;
            receiver = Expression::Send {
                receiver: Box::new(receiver),
                selector,
                arguments: vec![argument],
                kind: MessageKind::Binary,
                span: Span::new(start, end),
            };
        }
        Ok(receiver)
    }

    fn parse_unary_expression(&mut self) -> Result<Expression, ParseError> {
        let mut receiver = self.parse_primary()?;
        while let TokenKind::Identifier(selector) = &self.current().kind {
            let start = self.span_of_expr(&receiver).start;
            let selector = selector.clone();
            self.advance();
            let end = self.previous().span.end;
            receiver = Expression::Send {
                receiver: Box::new(receiver),
                selector,
                arguments: Vec::new(),
                kind: MessageKind::Unary,
                span: Span::new(start, end),
            };
        }
        Ok(receiver)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match &self.current().kind {
            TokenKind::Integer(value) => {
                let span = self.current().span;
                let value = *value;
                self.advance();
                Ok(Expression::Literal {
                    value: Literal::Integer(value),
                    span,
                })
            }
            TokenKind::String(value) => {
                let span = self.current().span;
                let value = value.clone();
                self.advance();
                Ok(Expression::Literal {
                    value: Literal::String(value),
                    span,
                })
            }
            TokenKind::Symbol(value) => {
                let span = self.current().span;
                let value = value.clone();
                self.advance();
                Ok(Expression::Literal {
                    value: Literal::Symbol(value),
                    span,
                })
            }
            TokenKind::HashLParen => {
                let start = self.current().span.start;
                let literal = self.parse_literal_array()?;
                let end = self.previous().span.end;
                Ok(Expression::Literal {
                    value: literal,
                    span: Span::new(start, end),
                })
            }
            TokenKind::Identifier(name) => {
                let span = self.current().span;
                let name = name.clone();
                self.advance();
                Ok(match name.as_str() {
                    "nil" => Expression::Literal {
                        value: Literal::Nil,
                        span,
                    },
                    "true" => Expression::Literal {
                        value: Literal::True,
                        span,
                    },
                    "false" => Expression::Literal {
                        value: Literal::False,
                        span,
                    },
                    "self" => Expression::PseudoVar {
                        value: PseudoVar::Self_,
                        span,
                    },
                    "super" => Expression::PseudoVar {
                        value: PseudoVar::Super,
                        span,
                    },
                    "thisContext" => Expression::PseudoVar {
                        value: PseudoVar::ThisContext,
                        span,
                    },
                    _ => Expression::Variable { name, span },
                })
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenKindMatcher::RParen, "expected )")?;
                Ok(expr)
            }
            TokenKind::LBracket => self.parse_block().map(Expression::Block),
            _ => Err(self.error_here("expected expression")),
        }
    }

    fn parse_literal_array(&mut self) -> Result<Literal, ParseError> {
        self.expect(
            TokenKindMatcher::HashLParen,
            "expected #( to start literal array",
        )?;
        let mut values = Vec::new();
        while !matches!(self.current().kind, TokenKind::RParen | TokenKind::End) {
            values.push(self.parse_literal()?);
        }
        self.expect(
            TokenKindMatcher::RParen,
            "expected ) to close literal array",
        )?;
        Ok(Literal::LiteralArray(values))
    }

    fn parse_literal(&mut self) -> Result<Literal, ParseError> {
        match &self.current().kind {
            TokenKind::Integer(value) => {
                let value = *value;
                self.advance();
                Ok(Literal::Integer(value))
            }
            TokenKind::String(value) => {
                let value = value.clone();
                self.advance();
                Ok(Literal::String(value))
            }
            TokenKind::Symbol(value) => {
                let value = value.clone();
                self.advance();
                Ok(Literal::Symbol(value))
            }
            TokenKind::HashLParen => self.parse_literal_array(),
            TokenKind::Identifier(name) if name == "nil" => {
                self.advance();
                Ok(Literal::Nil)
            }
            TokenKind::Identifier(name) if name == "true" => {
                self.advance();
                Ok(Literal::True)
            }
            TokenKind::Identifier(name) if name == "false" => {
                self.advance();
                Ok(Literal::False)
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Literal::Symbol(name))
            }
            _ => Err(self.error_here("expected literal")),
        }
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self
            .expect(TokenKindMatcher::LBracket, "expected [")?
            .span
            .start;
        let mut args = Vec::new();
        while matches!(self.current().kind, TokenKind::Colon) {
            self.advance();
            args.push(self.expect_identifier()?);
        }
        if !args.is_empty() {
            self.expect(TokenKindMatcher::Bar, "expected | after block args")?;
        }
        let temps = self.parse_temporaries()?;
        let statements = self.parse_statements(TokenKindMatcher::RBracket)?;
        let end = self
            .expect(TokenKindMatcher::RBracket, "expected ]")?
            .span
            .end;
        Ok(Block {
            args,
            temps,
            statements,
            span: Span::new(start, end),
        })
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.error_here("expected identifier")),
        }
    }

    fn expect(&mut self, matcher: TokenKindMatcher, message: &str) -> Result<Token, ParseError> {
        if matcher.matches(&self.current().kind) {
            let token = self.current().clone();
            self.advance();
            Ok(token)
        } else {
            Err(self.error_here(message))
        }
    }

    fn expect_end(&self) -> Result<(), ParseError> {
        if matches!(self.current().kind, TokenKind::End) {
            Ok(())
        } else {
            Err(self.error_here("expected end of input"))
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek(&self) -> &Token {
        &self.tokens[(self.pos + 1).min(self.tokens.len() - 1)]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos.saturating_sub(1)]
    }

    fn advance(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn error_here(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
            span: self.current().span,
        }
    }

    fn span_of_expr(&self, expr: &Expression) -> Span {
        match expr {
            Expression::Literal { span, .. }
            | Expression::Variable { span, .. }
            | Expression::PseudoVar { span, .. }
            | Expression::Send { span, .. }
            | Expression::Cascade { span, .. } => *span,
            Expression::Block(block) => block.span,
        }
    }
}

#[derive(Clone, Copy)]
enum TokenKindMatcher {
    End,
    RParen,
    RBracket,
    Bar,
    LBracket,
    HashLParen,
}

impl TokenKindMatcher {
    fn matches(self, kind: &TokenKind) -> bool {
        matches!(
            (self, kind),
            (Self::End, TokenKind::End)
                | (Self::RParen, TokenKind::RParen)
                | (Self::RBracket, TokenKind::RBracket)
                | (Self::Bar, TokenKind::Bar)
                | (Self::LBracket, TokenKind::LBracket)
                | (Self::HashLParen, TokenKind::HashLParen)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_doit, parse_expression, parse_method};
    use crate::compiler::ast::{Expression, Literal, MessageKind, Statement};

    #[test]
    fn parses_keyword_method_pattern() {
        let method = parse_method("at: index put: value | old | old := index. ^ value").unwrap();
        assert_eq!(method.pattern.selector, "at:put:");
        assert_eq!(method.pattern.arguments, vec!["index", "value"]);
        assert_eq!(method.temps, vec!["old"]);
    }

    #[test]
    fn parses_message_precedence() {
        let expr = parse_expression("a b + c d: e").unwrap();
        match expr {
            Expression::Send { selector, kind, .. } => {
                assert_eq!(selector, "d:");
                assert_eq!(kind, MessageKind::Keyword);
            }
            other => panic!("unexpected ast: {other:?}"),
        }
    }

    #[test]
    fn parses_blocks_with_args_and_temps() {
        let expr = parse_expression("[:x :y | | z | z := x + y. z]").unwrap();
        match expr {
            Expression::Block(block) => {
                assert_eq!(block.args, vec!["x", "y"]);
                assert_eq!(block.temps, vec!["z"]);
                assert_eq!(block.statements.len(), 2);
            }
            other => panic!("unexpected ast: {other:?}"),
        }
    }

    #[test]
    fn parses_literal_arrays() {
        let expr = parse_expression("#(1 'a' true foo #bar)").unwrap();
        match expr {
            Expression::Literal {
                value: Literal::LiteralArray(values),
                ..
            } => {
                assert_eq!(values.len(), 5);
            }
            other => panic!("unexpected ast: {other:?}"),
        }
    }

    #[test]
    fn parses_return_statement() {
        let method = parse_method("size ^ count").unwrap();
        assert!(matches!(method.statements[0], Statement::Return { .. }));
    }

    #[test]
    fn parses_doit_with_temps_and_assignments() {
        let doit = parse_doit("| x | x := 1. x + 2").unwrap();
        assert_eq!(doit.temps, vec!["x"]);
        assert_eq!(doit.statements.len(), 2);
        assert!(matches!(doit.statements[0], Statement::Assignment { .. }));
    }
}
