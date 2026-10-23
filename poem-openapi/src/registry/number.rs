use std::{cmp::Ordering, fmt, num::NonZero};

use serde::{Serialize, Serializer, ser::Error};

/// A numeric schema bound without rounding integer values through `f64`.
///
/// Integer bounds cover the full `i64` and `u64` ranges. Floating-point bounds
/// retain `f64` precision, not arbitrary decimal precision. Non-finite floats
/// cannot be serialized and do not satisfy numeric bound comparisons.
#[derive(Debug, Clone, Copy)]
pub enum MetaSchemaNumber {
    /// A signed integer bound.
    Integer(i64),
    /// An unsigned integer bound.
    Unsigned(u64),
    /// A floating-point bound.
    Float(f64),
}

impl MetaSchemaNumber {
    fn integer(self) -> Option<i128> {
        match self {
            Self::Integer(n) => Some(i128::from(n)),
            Self::Unsigned(n) => Some(i128::from(n)),
            Self::Float(_) => None,
        }
    }
}

// Every integer represented by MetaSchemaNumber is in [-2^63, 2^64). Check
// this range before casting the float, so saturation cannot erase ordering.
// Within the range, truncation is exact and only an equal integer needs the
// fractional part to break the tie. Never round the integer to a float.
fn compare_integer_float(integer: i128, float: f64) -> Option<Ordering> {
    if !float.is_finite() {
        return None;
    }
    if float < -9_223_372_036_854_775_808.0 {
        return Some(Ordering::Greater);
    }
    if float >= 18_446_744_073_709_551_616.0 {
        return Some(Ordering::Less);
    }
    match integer.cmp(&(float as i128)) {
        Ordering::Equal => 0.0_f64.partial_cmp(&float.fract()),
        ordering => Some(ordering),
    }
}

impl PartialOrd for MetaSchemaNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (Self::Float(left), Self::Float(right)) => {
                if left.is_finite() && right.is_finite() {
                    left.partial_cmp(&right)
                } else {
                    None
                }
            }
            (Self::Float(left), right) => {
                compare_integer_float(right.integer()?, left).map(Ordering::reverse)
            }
            (left, Self::Float(right)) => compare_integer_float(left.integer()?, right),
            (left, right) => left.integer()?.partial_cmp(&right.integer()?),
        }
    }
}

impl PartialEq for MetaSchemaNumber {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl fmt::Display for MetaSchemaNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(n) => n.fmt(f),
            Self::Unsigned(n) => n.fmt(f),
            Self::Float(n) => n.fmt(f),
        }
    }
}

impl Serialize for MetaSchemaNumber {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Integer(n) => serializer.serialize_i64(n),
            Self::Unsigned(n) => serializer.serialize_u64(n),
            Self::Float(n) if n.is_finite() => serializer.serialize_f64(n),
            Self::Float(_) => Err(S::Error::custom("numeric schema bounds must be finite")),
        }
    }
}

macro_rules! from_integer {
    ($variant:ident, $target:ty, $($ty:ty),*) => {
        $(
            impl From<$ty> for MetaSchemaNumber {
                fn from(value: $ty) -> Self {
                    Self::$variant(value as $target)
                }
            }

            impl From<NonZero<$ty>> for MetaSchemaNumber {
                fn from(value: NonZero<$ty>) -> Self {
                    value.get().into()
                }
            }
        )*
    };
}

from_integer!(Integer, i64, i8, i16, i32, i64, isize);
from_integer!(Unsigned, u64, u8, u16, u32, u64, usize);

impl From<f32> for MetaSchemaNumber {
    fn from(value: f32) -> Self {
        Self::Float(f64::from(value))
    }
}

impl From<f64> for MetaSchemaNumber {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}
