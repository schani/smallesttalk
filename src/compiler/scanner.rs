use crate::compiler::token::{Span, Token, TokenKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ScanError {}

pub fn scan(source: &str) -> Result<Vec<Token>, ScanError> {
    Scanner::new(source).scan_all()
}

struct Scanner<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
        }
    }

    fn scan_all(&mut self) -> Result<Vec<Token>, ScanError> {
        let mut tokens = Vec::new();
        while let Some(byte) = self.peek() {
            match byte {
                b' ' | b'\t' | b'\r' | b'\n' => {
                    self.pos += 1;
                }
                b'"' => self.skip_comment()?,
                b'(' => tokens.push(self.single(TokenKind::LParen)),
                b')' => tokens.push(self.single(TokenKind::RParen)),
                b'[' => tokens.push(self.single(TokenKind::LBracket)),
                b']' => tokens.push(self.single(TokenKind::RBracket)),
                b'|' => tokens.push(self.single(TokenKind::Bar)),
                b'.' => tokens.push(self.single(TokenKind::Period)),
                b';' => tokens.push(self.single(TokenKind::Semicolon)),
                b'^' => tokens.push(self.single(TokenKind::Return)),
                b':' => {
                    if self.peek_next() == Some(b'=') {
                        let start = self.pos;
                        self.pos += 2;
                        tokens.push(Token::new(TokenKind::Assign, Span::new(start, self.pos)));
                    } else {
                        tokens.push(self.single(TokenKind::Colon));
                    }
                }
                b'#' => tokens.push(self.scan_hash_literal()?),
                b'\'' => tokens.push(self.scan_string()?),
                b'0'..=b'9' => tokens.push(self.scan_integer()?),
                _ if is_identifier_start(byte) => tokens.push(self.scan_identifier_or_keyword()),
                _ if is_binary_char(byte) => tokens.push(self.scan_binary_selector()),
                _ => {
                    return Err(ScanError {
                        message: format!("unexpected character {:?}", byte as char),
                        span: Span::new(self.pos, self.pos + 1),
                    });
                }
            }
        }
        tokens.push(Token::new(TokenKind::End, Span::new(self.pos, self.pos)));
        Ok(tokens)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn single(&mut self, kind: TokenKind) -> Token {
        let start = self.pos;
        self.pos += 1;
        Token::new(kind, Span::new(start, self.pos))
    }

    fn skip_comment(&mut self) -> Result<(), ScanError> {
        let start = self.pos;
        self.pos += 1;
        while let Some(byte) = self.peek() {
            self.pos += 1;
            if byte == b'"' {
                return Ok(());
            }
        }
        Err(ScanError {
            message: "unterminated comment".to_string(),
            span: Span::new(start, self.pos),
        })
    }

    fn scan_integer(&mut self) -> Result<Token, ScanError> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        let text = &self.source[start..self.pos];
        let value = text.parse::<i64>().map_err(|_| ScanError {
            message: format!("invalid integer literal {text}"),
            span: Span::new(start, self.pos),
        })?;
        Ok(Token::new(
            TokenKind::Integer(value),
            Span::new(start, self.pos),
        ))
    }

    fn scan_identifier_or_keyword(&mut self) -> Token {
        let start = self.pos;
        self.pos += 1;
        while matches!(self.peek(), Some(byte) if is_identifier_continue(byte)) {
            self.pos += 1;
        }
        if self.peek() == Some(b':') && self.peek_next() != Some(b'=') {
            self.pos += 1;
            return Token::new(
                TokenKind::Keyword(self.source[start..self.pos].to_string()),
                Span::new(start, self.pos),
            );
        }
        Token::new(
            TokenKind::Identifier(self.source[start..self.pos].to_string()),
            Span::new(start, self.pos),
        )
    }

    fn scan_binary_selector(&mut self) -> Token {
        let start = self.pos;
        while matches!(self.peek(), Some(byte) if is_binary_char(byte)) {
            self.pos += 1;
        }
        Token::new(
            TokenKind::BinarySelector(self.source[start..self.pos].to_string()),
            Span::new(start, self.pos),
        )
    }

    fn scan_hash_literal(&mut self) -> Result<Token, ScanError> {
        let start = self.pos;
        self.pos += 1;
        match self.peek() {
            Some(b'(') => {
                self.pos += 1;
                Ok(Token::new(
                    TokenKind::HashLParen,
                    Span::new(start, self.pos),
                ))
            }
            Some(b'\'') => {
                let string = self.scan_string()?;
                let TokenKind::String(value) = string.kind else {
                    unreachable!()
                };
                Ok(Token::new(
                    TokenKind::Symbol(value),
                    Span::new(start, string.span.end),
                ))
            }
            Some(byte) if is_identifier_start(byte) => {
                let symbol_start = self.pos;
                self.pos += 1;
                while matches!(self.peek(), Some(byte) if is_identifier_continue(byte)) {
                    self.pos += 1;
                }
                while self.peek() == Some(b':') {
                    self.pos += 1;
                    while matches!(self.peek(), Some(byte) if is_identifier_continue(byte)) {
                        self.pos += 1;
                    }
                    if !matches!(self.peek(), Some(b':')) {
                        break;
                    }
                }
                let name = self.source[symbol_start..self.pos].to_string();
                Ok(Token::new(
                    TokenKind::Symbol(name),
                    Span::new(start, self.pos),
                ))
            }
            Some(byte) if is_binary_char(byte) => {
                let selector = self.scan_binary_selector();
                let TokenKind::BinarySelector(name) = selector.kind else {
                    unreachable!()
                };
                Ok(Token::new(
                    TokenKind::Symbol(name),
                    Span::new(start, selector.span.end),
                ))
            }
            _ => Err(ScanError {
                message: "invalid symbol literal".to_string(),
                span: Span::new(start, self.pos),
            }),
        }
    }

    fn scan_string(&mut self) -> Result<Token, ScanError> {
        let start = self.pos;
        self.pos += 1;
        let mut out = String::new();
        while let Some(byte) = self.peek() {
            self.pos += 1;
            if byte == b'\'' {
                if self.peek() == Some(b'\'') {
                    out.push('\'');
                    self.pos += 1;
                    continue;
                }
                return Ok(Token::new(
                    TokenKind::String(out),
                    Span::new(start, self.pos),
                ));
            }
            out.push(byte as char);
        }
        Err(ScanError {
            message: "unterminated string literal".to_string(),
            span: Span::new(start, self.pos),
        })
    }
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit()
}

fn is_binary_char(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'%'
            | b'&'
            | b'*'
            | b'+'
            | b','
            | b'/'
            | b'<'
            | b'='
            | b'>'
            | b'?'
            | b'@'
            | b'\\'
            | b'~'
            | b'-'
    )
}

#[cfg(test)]
mod tests {
    use super::scan;
    use crate::compiler::token::TokenKind;

    #[test]
    fn scans_basic_method_tokens() {
        let tokens = scan("at: index put: value | old | old := index + 1. ^ old").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Keyword(ref s) if s == "at:"));
        assert!(matches!(tokens[2].kind, TokenKind::Keyword(ref s) if s == "put:"));
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Assign))
        );
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::BinarySelector(ref s) if s == "+"))
        );
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Return))
        );
    }

    #[test]
    fn scans_symbol_and_literal_array_tokens() {
        let tokens = scan("#foo #(1 'a' true)").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Symbol(ref s) if s == "foo"));
        assert!(matches!(tokens[1].kind, TokenKind::HashLParen));
        assert!(matches!(tokens[2].kind, TokenKind::Integer(1)));
        assert!(matches!(tokens[3].kind, TokenKind::String(ref s) if s == "a"));
    }
}
