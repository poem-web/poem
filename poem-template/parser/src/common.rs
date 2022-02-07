use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, one_of},
    combinator::{map, recognize, value},
    error::{context, ParseError},
    multi::{fold_many0, many0},
    sequence::tuple,
    IResult, Parser,
};

use crate::{LineColumn, Span, Spanned};

pub(crate) type LocatedSpan<'a> = nom_locate::LocatedSpan<&'a str>;

pub(crate) fn position<'a, O, E: ParseError<LocatedSpan<'a>>, F>(
    parser: F,
) -> impl FnMut(LocatedSpan<'a>) -> IResult<LocatedSpan<'a>, Spanned<O>, E>
where
    F: Parser<LocatedSpan<'a>, O, E>,
{
    map(
        tuple((nom_locate::position, parser, nom_locate::position)),
        |(start, output, end)| Spanned {
            span: Span {
                start: LineColumn {
                    line: start.location_line() as usize,
                    column: start.get_column(),
                },
                end: LineColumn {
                    line: end.location_line() as usize,
                    column: end.get_column(),
                },
            },
            value: output,
        },
    )
}

pub(crate) fn map_span_to_str<'a, E: ParseError<LocatedSpan<'a>>, F>(
    parser: F,
) -> impl FnMut(LocatedSpan<'a>) -> IResult<LocatedSpan<'a>, &'a str, E>
where
    F: Parser<LocatedSpan<'a>, LocatedSpan<'a>, E>,
{
    map(parser, |s| *s.fragment())
}

pub(crate) fn sp(input: LocatedSpan) -> IResult<LocatedSpan, ()> {
    fold_many0(value((), one_of(" \t\n\r")), || (), |_, _| ())(input)
}

pub(crate) fn ident(input: LocatedSpan) -> IResult<LocatedSpan, Spanned<String>> {
    context(
        "ident",
        map(
            position(recognize(tuple((
                alt((alpha1, tag("_"))),
                many0(alt((alphanumeric1, tag("_")))),
            )))),
            |value| value.map(|value| value.to_string()),
        ),
    )(input)
}
