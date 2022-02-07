use std::str::FromStr;

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, tag_no_case, take},
    character::complete::{char, digit1},
    combinator::{cut, map, opt, recognize, value},
    error::context,
    multi::fold_many0,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

use crate::{
    common::{map_span_to_str, position, LocatedSpan},
    Spanned,
};

pub(crate) fn boolean(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<bool>> {
    context(
        "boolean",
        position(alt((
            map(tag_no_case("true"), |_| true),
            map(tag_no_case("false"), |_| false),
        ))),
    )(input)
}

pub(crate) fn integer(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<i64>> {
    context(
        "integer",
        position(map(
            recognize(tuple((opt(char('-')), digit1))),
            |s: LocatedSpan| i64::from_str(s.fragment()).unwrap(),
        )),
    )(input)
}

pub(crate) fn float(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<f64>> {
    context(
        "float",
        position(map(
            recognize(tuple((
                opt(char('-')),
                alt((
                    map(tuple((digit1, pair(char('.'), opt(digit1)))), |_| ()),
                    map(tuple((char('.'), digit1)), |_| ()),
                )),
                opt(tuple((
                    alt((char('e'), char('E'))),
                    opt(alt((char('+'), char('-')))),
                    cut(digit1),
                ))),
            ))),
            |s: LocatedSpan| f64::from_str(s.fragment()).unwrap(),
        )),
    )(input)
}

pub(crate) fn string(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<String>> {
    position(delimited(
        tag("\""),
        fold_many0(
            alt((
                map_span_to_str(is_not("\\\"")),
                value("\"", tag("\"\"")),
                value("\\", tag("\\\\")),
                value("\x7f", tag("\\b")),
                value("\r", tag("\\r")),
                value("\n", tag("\\n")),
                value("\t", tag("\\t")),
                value("\0", tag("\\0")),
                value("\x1A", tag("\\Z")),
                map_span_to_str(preceded(tag("\\"), take(1usize))),
            )),
            String::new,
            |mut acc: String, s: &str| {
                acc.push_str(s);
                acc
            },
        ),
        tag("\""),
    ))(input)
}
