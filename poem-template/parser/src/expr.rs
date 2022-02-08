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
        alt((
            map(position(tag_no_case("null")), |s| Spanned {
                span: s.span,
                value: Expr::Literal(s.map(|_| Literal::Null)),
            }),
            map(boolean, |s| Spanned {
                span: s.span,
                value: Expr::Literal(s.map(Literal::Boolean)),
            }),
            map(float, |s| Spanned {
                span: s.span,
                value: Expr::Literal(s.map(Literal::Float)),
            }),
            map(integer, |s| Spanned {
                span: s.span,
                value: Expr::Literal(s.map(Literal::Integer)),
            }),
            map(string, |s| Spanned {
                span: s.span,
                value: Expr::Literal(s.map(Literal::String)),
            }),
        )),
    )(input)
}

fn variable(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    context(
        "variable",
        map(ident, |s| Spanned {
            span: s.span,
            value: Expr::Variable(s),
        }),
    )(input)
}

fn expr_parens(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    map(
        tuple((position(char('(')), sp, expr, sp, position(char(')')))),
        |(s, _, expr, _, e)| Spanned {
            span: Span::new(s.span.start, e.span.end),
            value: Expr::Group(expr.map(Box::new)),
        },
    )(input)
}

fn expr_primitive(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let expr = alt((expr_parens, expr_unary, variable, literal));
    context("expr_primitive", delimited(sp, expr, sp))(input)
}

fn expr_unary(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Expr>> {
    let op = position(alt((
        value(UnaryOperator::Not, tag_no_case("not")),
        value(UnaryOperator::Neg, char('-')),
    )));
    map(separated_pair(op, sp, expr), |(op, expr)| {
        let span = Span::new(op.span.start, expr.span.end);
        Spanned {
            span,
            value: Expr::Unary(Spanned {
                span,
                value: UnaryExpr {
                    op,
                    expr: expr.map(Box::new),
                },
            }),
        }
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
    rem.into_iter().fold(expr, |lhs, (op, rhs)| {
        let span = Span::new(lhs.span.start, rhs.span.end);
        Spanned {
            span,
            value: Expr::Binary(Spanned {
                span,
                value: BinaryExpr {
                    op,
                    lhs: lhs.map(Box::new),
                    rhs: rhs.map(Box::new),
                },
            }),
        }
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
        check_spanned!(
            literal,
            r#"true"#,
            Expr::Literal(Spanned::new(Literal::Boolean(true)))
        );
        check_spanned!(
            literal,
            r#"0"#,
            Expr::Literal(Spanned::new(Literal::Integer(0)))
        );
        check_spanned!(
            literal,
            r#"127"#,
            Expr::Literal(Spanned::new(Literal::Integer(127)))
        );
        check_spanned!(
            literal,
            r#"-128"#,
            Expr::Literal(Spanned::new(Literal::Integer(-128)))
        );
        check_spanned!(
            literal,
            r#""abc""#,
            Expr::Literal(Spanned::new(Literal::String("abc".to_string())))
        );
    }

    #[test]
    fn test_variable() {
        check_spanned!(
            variable,
            r#"abc"#,
            Expr::Variable(Spanned::new("abc".to_string()))
        );
    }

    #[test]
    fn test_expr() {
        check_spanned!(
            expr,
            r#"2000+4/2"#,
            Expr::Binary(Spanned::new(BinaryExpr {
                op: Spanned::new(BinaryOperator::Plus),
                lhs: Spanned::new(Box::new(Expr::Literal(Spanned::new(Literal::Integer(
                    2000
                ))))),
                rhs: Spanned::new(Box::new(Expr::Binary(Spanned::new(BinaryExpr {
                    op: Spanned::new(BinaryOperator::Divide),
                    lhs: Spanned::new(Box::new(Expr::Literal(Spanned::new(Literal::Integer(4))))),
                    rhs: Spanned::new(Box::new(Expr::Literal(Spanned::new(Literal::Integer(2)))))
                }))))
            }))
        );

        check_spanned!(
            expr,
            r#"(2000+4)/2"#,
            Expr::Binary(Spanned::new(BinaryExpr {
                op: Spanned::new(BinaryOperator::Divide),
                lhs: Spanned::new(Box::new(Expr::Group(Spanned::new(Box::new(Expr::Binary(
                    Spanned::new(BinaryExpr {
                        op: Spanned::new(BinaryOperator::Plus),
                        lhs: Spanned::new(Box::new(Expr::Literal(Spanned::new(Literal::Integer(
                            2000
                        ))))),
                        rhs: Spanned::new(Box::new(Expr::Literal(Spanned::new(Literal::Integer(
                            4
                        )))))
                    })
                )))))),
                rhs: Spanned::new(Box::new(Expr::Literal(Spanned::new(Literal::Integer(2))))),
            }))
        );
    }
}
