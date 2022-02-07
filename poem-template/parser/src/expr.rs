use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::char,
    combinator::{map, value},
    error::context,
    multi::many0,
    sequence::{delimited, separated_pair, tuple},
    IResult,
};

use crate::{
    ast::{BinaryExpr, BinaryOperator, UnaryExpr, UnaryOperator},
    common::{ident, position, sp, LocatedSpan},
    constant::{boolean, float, integer, string},
    Expr, Literal, Span, Spanned,
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

fn expr_parens(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    map(
        tuple((char('('), sp, expr, sp, char(')'))),
        |(_, _, expr, _, _)| expr.map(|expr| Expr::Group(Box::new(expr))),
    )(input)
}

fn expr_primitive(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let expr = alt((expr_parens, variable, literal));
    context("expr_primitive", delimited(sp, expr, sp))(input)
}

fn expr_unary(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let op = position(alt((
        value(UnaryOperator::Not, tag_no_case("not")),
        value(UnaryOperator::Neg, char('-')),
    )));
    map(separated_pair(op, sp, expr), |(op, expr)| Spanned {
        span: Span::new(op.span.start, expr.span.end),
        value: Expr::Unary(UnaryExpr {
            op,
            expr: expr.map(Box::new),
        }),
    })(input)
}

fn expr_binary_1(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let (input, lhs) = expr_binary_2(input)?;
    let (input, exprs) = many0(tuple((
        position(value(BinaryOperator::Or, tag_no_case("or"))),
        expr_binary_2,
    )))(input)?;
    Ok((input, parse_expr(lhs, exprs)))
}

fn expr_binary_2(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let (input, lhs) = expr_binary_3(input)?;
    let (input, exprs) = many0(tuple((
        position(value(BinaryOperator::And, tag_no_case("and"))),
        expr_binary_3,
    )))(input)?;
    Ok((input, parse_expr(lhs, exprs)))
}

fn expr_binary_3(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let (input, lhs) = expr_binary_4(input)?;
    let (input, exprs) = many0(tuple((
        position(alt((
            value(BinaryOperator::Eq, tag("=")),
            value(BinaryOperator::NotEq, tag("!=")),
            value(BinaryOperator::NotEq, tag("<>")),
            value(BinaryOperator::Lt, tag("<")),
            value(BinaryOperator::LtEq, tag("<")),
            value(BinaryOperator::Gt, tag(">")),
            value(BinaryOperator::GtEq, tag(">=")),
        ))),
        expr_binary_4,
    )))(input)?;
    Ok((input, parse_expr(lhs, exprs)))
}

fn expr_binary_4(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let (input, lhs) = expr_binary_5(input)?;
    let (input, exprs) = many0(tuple((
        position(alt((
            value(BinaryOperator::Plus, char('+')),
            value(BinaryOperator::Minus, char('-')),
        ))),
        expr_binary_5,
    )))(input)?;
    Ok((input, parse_expr(lhs, exprs)))
}

fn expr_binary_5(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let (input, lhs) = expr_primitive(input)?;
    let (input, exprs) = many0(tuple((
        position(alt((
            value(BinaryOperator::Multiply, char('*')),
            value(BinaryOperator::Divide, char('/')),
            value(BinaryOperator::Rem, char('%')),
        ))),
        expr_primitive,
    )))(input)?;
    Ok((input, parse_expr(lhs, exprs)))
}

fn parse_expr(
    expr: Spanned<Expr>,
    rem: Vec<(Spanned<BinaryOperator>, Spanned<Expr>)>,
) -> Spanned<Expr> {
    rem.into_iter().fold(expr, |lhs, (op, rhs)| Spanned {
        span: Span::new(lhs.span.start, rhs.span.end),
        value: Expr::Binary(BinaryExpr {
            op,
            lhs: lhs.map(Box::new),
            rhs: rhs.map(Box::new),
        }),
    })
}

pub(crate) fn expr(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    context("expr", expr_binary_1)(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_literal() {
        check_spanned!(literal, r#"true"#, Expr::Literal(Literal::Boolean(true)));
        check_spanned!(literal, r#"0"#, Expr::Literal(Literal::Integer(0)));
        check_spanned!(literal, r#"127"#, Expr::Literal(Literal::Integer(127)));
        check_spanned!(literal, r#"-128"#, Expr::Literal(Literal::Integer(-128)));
        check_spanned!(
            literal,
            r#""abc""#,
            Expr::Literal(Literal::String("abc".to_string()))
        );
    }

    #[test]
    fn test_variable() {
        check_spanned!(variable, r#"abc"#, Expr::Variable("abc".to_string()));
    }

    #[test]
    fn test_expr() {
        check_spanned!(
            expr,
            r#"2000+4/2"#,
            Expr::Binary(BinaryExpr {
                op: Spanned::new(BinaryOperator::Plus),
                lhs: Spanned::new(Box::new(Expr::Literal(Literal::Integer(2000)))),
                rhs: Spanned::new(Box::new(Expr::Binary(BinaryExpr {
                    op: Spanned::new(BinaryOperator::Divide),
                    lhs: Spanned::new(Box::new(Expr::Literal(Literal::Integer(4)))),
                    rhs: Spanned::new(Box::new(Expr::Literal(Literal::Integer(2))))
                })))
            })
        );

        check_spanned!(
            expr,
            r#"(2000+4)/2"#,
            Expr::Binary(BinaryExpr {
                op: Spanned::new(BinaryOperator::Divide),
                lhs: Spanned::new(Box::new(Expr::Group(Box::new(Expr::Binary(BinaryExpr {
                    op: Spanned::new(BinaryOperator::Plus),
                    lhs: Spanned::new(Box::new(Expr::Literal(Literal::Integer(2000)))),
                    rhs: Spanned::new(Box::new(Expr::Literal(Literal::Integer(4))))
                }))))),
                rhs: Spanned::new(Box::new(Expr::Literal(Literal::Integer(2)))),
            })
        );
    }
}
