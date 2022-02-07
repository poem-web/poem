use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    combinator::map,
    error::{context, ParseError},
    multi::many0,
    sequence::delimited,
    FindSubstring, IResult, InputLength, InputTake,
};

use crate::{
    common::{position, sp, LocatedSpan},
    expr::expr,
    Block, Expr, Node, Spanned,
};

fn take_raw<T, Input, Error: ParseError<Input>>(
    tag: T,
) -> impl Fn(Input) -> IResult<Input, Input, Error>
where
    Input: InputTake + FindSubstring<T> + InputLength,
    T: InputLength + Clone,
{
    move |i: Input| {
        let t = tag.clone();
        let res: IResult<_, _, Error> = match i.find_substring(t) {
            None => Ok(i.take_split(i.input_len())),
            Some(index) => Ok(i.take_split(index)),
        };
        res
    }
}

pub(crate) fn node_raw(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Node>> {
    context(
        "node_raw",
        map(position(take_raw("{{")), |value| {
            value.map(|value| Node::Raw(value.to_string()))
        }),
    )(input)
}

fn node_expr(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Node>> {
    context(
        "node_expr",
        map(delimited(sp, expr, sp), |expr| expr.map(Node::Expr)),
    )(input)
}

fn node_if(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Node>> {
    context("node_if", tag_no_case("if"))(input)
}

fn node_tag(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<Node>> {
    context(
        "node_tag",
        delimited(tag("{{"), alt((node_expr, node_if)), tag("}}")),
    )(input)
}

pub(crate) fn block(input: LocatedSpan) -> IResult<LocatedSpan, Block> {
    context(
        "block",
        map(many0(alt((node_raw, node_tag))), |nodes| Block { nodes }),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_raw() {
        check_spanned!(
            node_raw,
            "  2 ab ssd {{ df",
            Node::Raw("  2 ab ssd ".to_string())
        );

        check_spanned!(
            node_raw,
            "  2 ab ssd df",
            Node::Raw("  2 ab ssd df".to_string())
        );

        check_spanned!(
            node_raw,
            "  2 ab s{ {sd df",
            Node::Raw("  2 ab s{ {sd df".to_string())
        );
    }
}
