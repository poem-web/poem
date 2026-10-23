use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, Lit, UnOp};

#[derive(Clone)]
pub(crate) enum NumericBound {
    Integer(i64),
    Unsigned(u64),
    Float(f64),
}

impl NumericBound {
    fn from_literal(value: &Lit, negative: bool) -> darling::Result<Self> {
        let result = match value {
            Lit::Int(value) => {
                // Do not fall back to f64 if an integer is outside the
                // supported range: that would silently turn a
                // precise bound into a rounded one.
                let digits = value.base10_digits();
                // Darling can fold a negative attribute expression into its
                // literal.
                let negative = negative ^ digits.starts_with('-');
                let magnitude = digits
                    .strip_prefix('-')
                    .unwrap_or(digits)
                    .parse::<u64>()
                    .map_err(|_| darling::Error::custom("integer bound is above u64::MAX"))?;
                if negative {
                    i64::try_from(-i128::from(magnitude))
                        .map(Self::Integer)
                        .map_err(|_| darling::Error::custom("integer bound is below i64::MIN"))
                } else {
                    Ok(Self::Unsigned(magnitude))
                }
            }
            Lit::Float(value) => {
                let number = value.base10_parse::<f64>().map_err(darling::Error::from)?;
                if number.is_finite() {
                    Ok(Self::Float(if negative { -number } else { number }))
                } else {
                    Err(darling::Error::custom("numeric bounds must be finite"))
                }
            }
            _ => Err(darling::Error::unexpected_lit_type(value)),
        };
        result.map_err(|error| error.with_span(value))
    }
}

impl FromMeta for NumericBound {
    fn from_string(value: &str) -> darling::Result<Self> {
        // Keep accepting the quoted numeric values used by existing attributes.
        let value = value.strip_prefix('+').unwrap_or(value);
        if value.contains(['.', 'e', 'E'])
            && let Ok(number) = value.parse::<f64>()
        {
            return if number.is_finite() {
                Ok(Self::Float(number))
            } else {
                Err(darling::Error::custom("numeric bounds must be finite"))
            };
        }
        Self::from_expr(&syn::parse_str::<Expr>(value)?)
    }

    fn from_value(value: &Lit) -> darling::Result<Self> {
        if let Lit::Str(value) = value {
            Self::from_string(&value.value()).map_err(|error| error.with_span(value))
        } else {
            Self::from_literal(value, false)
        }
    }

    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        match expr {
            Expr::Lit(value) => Self::from_value(&value.lit),
            Expr::Unary(value) if matches!(value.op, UnOp::Neg(_)) => {
                if let Expr::Lit(literal) = &*value.expr {
                    Self::from_literal(&literal.lit, true)
                } else {
                    Err(darling::Error::unexpected_expr_type(expr))
                }
            }
            Expr::Group(value) => Self::from_expr(&value.expr),
            _ => Err(darling::Error::unexpected_expr_type(expr)),
        }
        .map_err(|error| error.with_span(expr))
    }
}

impl ToTokens for NumericBound {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::Integer(value) => quote!(#value),
            Self::Unsigned(value) => quote!(#value),
            Self::Float(value) => quote!(#value),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_integer_tokens() {
        for (input, expected) in [
            ("9007199254740993", "9007199254740993u64"),
            ("18446744073709551615", "18446744073709551615u64"),
            ("-9223372036854775808", "- 9223372036854775808i64"),
            ("0xffff_ffff_ffff_ffff", "18446744073709551615u64"),
        ] {
            let number = NumericBound::from_string(input).unwrap();
            assert_eq!(number.to_token_stream().to_string(), expected);
        }
    }

    #[test]
    fn rejects_out_of_range_and_non_finite_bounds() {
        for input in [
            "18446744073709551616",
            "-9223372036854775809",
            "1e309",
            "-1e309",
            "NaN",
            "inf",
            "-inf",
            "true",
        ] {
            assert!(NumericBound::from_string(input).is_err(), "{input}");
        }
    }

    #[test]
    fn accepts_fractional_and_exponent_bounds() {
        for (input, expected) in [
            ("-1.5", -1.5),
            ("1e3", 1000.0),
            ("+1.5", 1.5),
            (".5", 0.5),
            ("-.5", -0.5),
        ] {
            let NumericBound::Float(value) = NumericBound::from_string(input).unwrap() else {
                panic!("expected float");
            };
            assert_eq!(value, expected);
        }
    }
}
