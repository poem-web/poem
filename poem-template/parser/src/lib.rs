pub struct Pos {
    pub line: usize,
    pub column: usize,
}

pub struct Spanned<T> {
    pub pos: Pos,
    pub value: T,
}

pub enum Node {
    Raw(Spanned<String>),
    Expr(Spanned<Expr>),
}

pub enum Literal {
    Boolean(Spanned<bool>),
    Integer(Spanned<i64>),
    Float(Spanned<f64>),
    String(Spanned<String>),
    Variable(Spanned<String>),
}

pub enum Expr {
    Literal(Spanned<Literal>),
}
