#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[inline]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    #[inline]
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Keyword(String),
    BinarySelector(String),
    Integer(i64),
    String(String),
    Symbol(String),
    HashLParen,
    Assign,
    Return,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Bar,
    Period,
    Semicolon,
    Colon,
    End,
}
