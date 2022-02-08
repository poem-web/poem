use nom::combinator::opt;
use nom::sequence::preceded;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    combinator::map,
    error::{context, ErrorKind, ParseError},
    multi::many0,
    sequence::tuple,
    FindSubstring, IResult, InputLength, InputTake,
};

use crate::{
    common::{position, sp, LocatedSpan},
    expr::expr,
    Block, IfNode, Node, Span, Spanned,
};

fn take_raw<T, Input, Error: ParseError<Input>>(
    tag: [T; 2],
) -> impl Fn(Input) -> IResult<Input, Input, Error>
where
    Input: InputTake + FindSubstring<T> + InputLength,
    T: InputLength + Clone,
{
    move |i: Input| {
        let t = tag.clone();
        if i.input_len() == 0 {
            return Err(nom::Err::Error(Error::from_error_kind(
                i,
                ErrorKind::Complete,
            )));
        }
        let res: IResult<_, _, Error> = match t.into_iter().find_map(|t| i.find_substring(t)) {
            None => Ok(i.take_split(i.input_len())),
            Some(0) => Err(nom::Err::Error(Error::from_error_kind(
                i,
                ErrorKind::Complete,
            ))),
            Some(index) => Ok(i.take_split(index)),
        };
        res
    }
}

fn node_raw(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Node>> {
    context(
        "node_raw",
        map(position(take_raw(["{{", "{%"])), |value| Spanned {
            span: value.span,
            value: Node::Raw(Spanned {
                span: value.span,
                value: value.to_string(),
            }),
        }),
    )(input)
}

fn node_expr(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Node>> {
    context(
        "node_expr",
        map(
            tuple((position(tag("{{")), sp, expr, sp, position(tag("}}")))),
            |(s, _, expr, _, e)| Spanned {
                span: Span::new(s.span.start, e.span.end),
                value: Node::Expr(expr),
            },
        ),
    )(input)
}

fn node_if(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Node>> {
    let if_tag = map(
        tuple((tag("{%"), sp, tag_no_case("if"), sp, expr, sp, tag("%}"))),
        |(_, _, _, _, cond, _, _)| cond,
    );
    let end_tag = tuple((tag("{%"), sp, tag_no_case("end"), sp, tag("%}")));
    let else_tag = tuple((tag("{%"), sp, tag_no_case("else"), sp, tag("%}")));

    context(
        "node_if",
        map(
            tuple((
                if_tag,
                block,
                opt(preceded(else_tag, block)),
                position(end_tag),
            )),
            |(condition, then_block, else_block, end_tag)| {
                let span = Span::new(condition.span.start, end_tag.span.end);
                Spanned {
                    span,
                    value: Node::If(Spanned {
                        span,
                        value: IfNode {
                            condition,
                            then: then_block,
                            r#else: else_block,
                        },
                    }),
                }
            },
        ),
    )(input)
}

pub(crate) fn block(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Block>> {
    context(
        "block",
        position(map(many0(alt((node_if, node_expr, node_raw))), |nodes| {
            Block { nodes }
        })),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_node_raw() {
        check_spanned!(
            node_raw,
            "  2 ab ssd {{ df",
            Node::Raw(Spanned::new("  2 ab ssd ".to_string()))
        );

        check_spanned!(
            node_raw,
            "  2 ab ssd df",
            Node::Raw(Spanned::new("  2 ab ssd df".to_string()))
        );

        check_spanned!(
            node_raw,
            "  2 ab s{ {sd df",
            Node::Raw(Spanned::new("  2 ab s{ {sd df".to_string()))
        );
    }

    #[test]
    fn test_if() {
        check_spanned!(
            node_if,
            "{% if a %}abc{%end%}",
            Node::If(Spanned::new(IfNode {
                condition: Spanned::new(Expr::Variable(Spanned::new("a".to_string()))),
                then: Spanned::new(Block {
                    nodes: vec![Spanned::new(Node::Raw(Spanned::new("abc".to_string())))]
                }),
                r#else: None,
            }))
        );

        check_spanned!(
            node_if,
            "{% if a %}abc{%else%}def{%end%}",
            Node::If(Spanned::new(IfNode {
                condition: Spanned::new(Expr::Variable(Spanned::new("a".to_string()))),
                then: Spanned::new(Block {
                    nodes: vec![Spanned::new(Node::Raw(Spanned::new("abc".to_string())))]
                }),
                r#else: Some(Spanned::new(Block {
                    nodes: vec![Spanned::new(Node::Raw(Spanned::new("def".to_string())))]
                })),
            }))
        );

        check_spanned!(
            node_if,
            "{% if a %}abc{% if b %}rty{%else%}yui{%end%}{%else%}def{%end%}",
            Node::If(Spanned::new(IfNode {
                condition: Spanned::new(Expr::Variable(Spanned::new("a".to_string()))),
                then: Spanned::new(Block {
                    nodes: vec![
                        Spanned::new(Node::Raw(Spanned::new("abc".to_string()))),
                        Spanned::new(Node::If(Spanned::new(IfNode {
                            condition: Spanned::new(Expr::Variable(Spanned::new("b".to_string()))),
                            then: Spanned::new(Block {
                                nodes: vec![Spanned::new(Node::Raw(Spanned::new(
                                    "rty".to_string()
                                )))]
                            }),
                            r#else: Some(Spanned::new(Block {
                                nodes: vec![Spanned::new(Node::Raw(Spanned::new(
                                    "yui".to_string()
                                )))]
                            })),
                        })))
                    ]
                }),
                r#else: Some(Spanned::new(Block {
                    nodes: vec![Spanned::new(Node::Raw(Spanned::new("def".to_string())))]
                })),
            }))
        );
    }

    #[test]
    fn test_block() {
        check_spanned!(
            block,
            "  123 {{ a+1 }} bdef",
            Block {
                nodes: vec![
                    Spanned::new(Node::Raw(Spanned::new("  123 ".to_string()))),
                    Spanned::new(Node::Expr(Spanned::new(Expr::Binary(Spanned::new(
                        BinaryExpr {
                            op: Spanned::new(BinaryOperator::Plus),
                            lhs: Spanned::new(Box::new(Expr::Variable(Spanned::new(
                                "a".to_string()
                            )))),
                            rhs: Spanned::new(Box::new(Expr::Literal(Spanned::new(
                                Literal::Integer(1)
                            )))),
                        }
                    ))))),
                    Spanned::new(Node::Raw(Spanned::new(" bdef".to_string())))
                ]
            }
        );

        check_spanned!(
            block,
            "{{a}}",
            Block {
                nodes: vec![Spanned::new(Node::Expr(Spanned::new(Expr::Variable(
                    Spanned::new("a".to_string())
                ))))]
            }
        );
    }
}
