use crate::compiler::token::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    Integer(i64),
    String(String),
    Symbol(String),
    LiteralArray(Vec<Literal>),
    Nil,
    True,
    False,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoVar {
    Self_,
    Super,
    ThisContext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Unary,
    Binary,
    Keyword,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub args: Vec<String>,
    pub temps: Vec<String>,
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expression {
    Literal {
        value: Literal,
        span: Span,
    },
    Variable {
        name: String,
        span: Span,
    },
    PseudoVar {
        value: PseudoVar,
        span: Span,
    },
    Send {
        receiver: Box<Expression>,
        selector: String,
        arguments: Vec<Expression>,
        kind: MessageKind,
        span: Span,
    },
    Cascade {
        head: Box<Expression>,
        messages: Vec<CascadeMessage>,
        span: Span,
    },
    Block(Block),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CascadeMessage {
    pub selector: String,
    pub arguments: Vec<Expression>,
    pub kind: MessageKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    Expression(Expression),
    Assignment {
        name: String,
        value: Expression,
        span: Span,
    },
    Return {
        value: Expression,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodPattern {
    pub selector: String,
    pub arguments: Vec<String>,
    pub kind: MessageKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodDef {
    pub pattern: MethodPattern,
    pub temps: Vec<String>,
    pub statements: Vec<Statement>,
    pub span: Span,
}
