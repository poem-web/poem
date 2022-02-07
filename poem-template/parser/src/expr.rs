use nom::{
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::char,
    combinator::{map, value},
    error::context,
    sequence::{delimited, tuple},
    IResult,
};

use crate::{
    common::{ident, position, sp, LocatedSpan},
    constant::{boolean, float, integer, string},
    Expr, Literal, Spanned,
};

fn literal(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    context(
        "literal",
        map(
            alt((
                position(value(Literal::Null, tag_no_case("null"))),
                map(boolean, |s| s.map(Literal::Boolean)),
                map(float, |s| s.map(Literal::Float)),
                map(integer, |s| s.map(Literal::Integer)),
                map(string, |s| s.map(Literal::String)),
            )),
            |literal| literal.map(Expr::Literal),
        ),
    )(input)
}

fn variable(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    context("variable", map(ident, |s| s.map(Expr::Variable)))(input)
}

fn expr_primitive(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let parens = map(
        tuple((char('('), sp, expr, sp, char(')'))),
        |(_, _, expr, _, _)| expr.wrap(|expr| Expr::Group(expr.map(Box::new))),
    );
    let expr = alt((parens, variable));
    context("expr_primitive", delimited(sp, expr, sp))(input)
}
