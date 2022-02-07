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
    Literal(Literal),
    Variable(String),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Index(IndexExpr),
    Attr(AttrExpr),
    Filter(FilterExpr),
    Group(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    pub condition: Spanned<Expr>,
    pub then: Spanned<Block>,
    pub r#else: Option<Spanned<Block>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Raw(String),
    Expr(Expr),
    If(IfNode),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub nodes: Vec<Spanned<Node>>,
}
