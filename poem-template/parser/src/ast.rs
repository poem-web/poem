use crate::Spanned;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterExpr {
    pub expr: Spanned<Box<Expr>>,
    pub name: Spanned<String>,
    pub arguments: Vec<Spanned<Box<Expr>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexExpr {
    pub expr: Spanned<Box<Expr>>,
    pub index: Spanned<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttrExpr {
    pub expr: Spanned<Box<Expr>>,
    pub attr: Spanned<String>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BinaryOperator {
    And,
    Or,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Plus,
    Minus,
    Multiply,
    Divide,
    Rem,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr {
    pub op: Spanned<BinaryOperator>,
    pub lhs: Spanned<Box<Expr>>,
    pub rhs: Spanned<Box<Expr>>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum UnaryOperator {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr {
    pub op: Spanned<UnaryOperator>,
    pub expr: Spanned<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Spanned<Literal>),
    Variable(Spanned<String>),
    Binary(Spanned<BinaryExpr>),
    Unary(Spanned<UnaryExpr>),
    Index(Spanned<IndexExpr>),
    Attr(Spanned<AttrExpr>),
    Filter(Spanned<FilterExpr>),
    Group(Spanned<Box<Expr>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    pub condition: Spanned<Expr>,
    pub then: Spanned<Block>,
    pub r#else: Option<Spanned<Block>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForeachNode {
    pub var: Spanned<String>,
    pub source: Spanned<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Raw(Spanned<String>),
    Expr(Spanned<Expr>),
    If(Spanned<IfNode>),
    Foreach(Spanned<ForeachNode>),
    Block(Spanned<Block>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub nodes: Vec<Spanned<Node>>,
}
