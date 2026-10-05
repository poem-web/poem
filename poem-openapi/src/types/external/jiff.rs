use std::borrow::Cow;

use jiff::{
    Timestamp, Zoned,
    civil::{Date, DateTime, Time},
};
use poem::web::Field;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type,
    },
};

macro_rules! impl_jiff_type {
    ($ty:ty, $type_name:literal, $format:literal) => {
        impl Type for $ty {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> Cow<'static, str> {
                concat!($type_name, "_", $format).into()
            }

            fn schema_ref() -> MetaSchemaRef {
                MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format($type_name, $format)))
            }

            fn as_raw_value(&self) -> Option<&Self::RawValueType> {
                Some(self)
            }

            fn raw_element_iter<'a>(
                &'a self,
            ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
                Box::new(self.as_raw_value().into_iter())
            }
        }

        impl ParseFromJSON for $ty {
            fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
                // Distinguish "field absent" (expected_input) from "field present
                // but wrong JSON type" (expected_type)
                let value = value.ok_or_else(ParseError::expected_input)?;

                if let Value::String(s) = value {
                    Ok(s.parse()?)
                } else {
                    Err(ParseError::expected_type(value))
                }
            }
        }

        impl ParseFromParameter for $ty {
            fn parse_from_parameter(value: &str) -> ParseResult<Self> {
                Ok(value.parse()?)
            }
        }

        impl ParseFromMultipartField for $ty {
            async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
                match field {
                    Some(field) => Ok(field.text().await?.parse()?),
                    None => Err(ParseError::expected_input()),
                }
            }
        }

        impl ToJSON for $ty {
            fn to_json(&self) -> Option<Value> {
                Some(Value::String(self.to_string()))
            }
        }
    };
}

impl_jiff_type!(Timestamp, "string", "date-time");
// Zoned includes a time zone annotation, which is not RFC 3339 `date-time`.
impl_jiff_type!(Zoned, "string", "zoned-date-time");
impl_jiff_type!(DateTime, "string", "naive-date-time");
impl_jiff_type!(Date, "string", "naive-date");
impl_jiff_type!(Time, "string", "naive-time");

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use jiff::civil;
    use serde_json::json;

    use super::*;

    #[test]
    fn timestamp() {
        let ts = civil::date(2015, 9, 18)
            .at(23, 56, 4, 0)
            .in_tz("UTC")
            .unwrap()
            .timestamp();

        let value = ts.to_json();

        assert_eq!(
            value,
            Some(Value::String("2015-09-18T23:56:04Z".to_string()))
        );
        assert_eq!(
            Timestamp::parse_from_json(Some(Value::String("2015-09-18T23:56:04Z".to_string())))
                .unwrap(),
            civil::date(2015, 9, 18)
                .at(23, 56, 4, 0)
                .in_tz("UTC")
                .unwrap()
                .timestamp()
        );
    }

    #[test]
    fn zoned() {
        let zdt = civil::date(2015, 9, 18)
            .at(23, 56, 4, 0)
            .in_tz("UTC")
            .unwrap();
        let value = zdt.to_json();
        assert_eq!(
            value,
            Some(Value::String("2015-09-18T23:56:04+00:00[UTC]".to_string()))
        );
        assert_eq!(
            Zoned::parse_from_json(Some(Value::String(
                "2015-09-18T23:56:04+00:00[UTC]".to_string()
            )))
            .unwrap(),
            civil::date(2015, 9, 18)
                .at(23, 56, 4, 0)
                .in_tz("UTC")
                .unwrap()
        );
    }

    #[test]
    fn civil_datetime() {
        let dt = civil::date(2015, 9, 18).at(23, 56, 4, 0);
        let value = dt.to_json();
        assert_eq!(
            value,
            Some(Value::String("2015-09-18T23:56:04".to_string()))
        );
        assert_eq!(
            DateTime::parse_from_json(Some(Value::String("2015-09-18T23:56:04".to_string())))
                .unwrap(),
            civil::date(2015, 9, 18).at(23, 56, 4, 0)
        );
    }

    #[test]
    fn civil_date() {
        let date = civil::date(2015, 9, 18);
        let value = date.to_json();
        assert_eq!(value, Some(Value::String("2015-09-18".to_string())));
        assert_eq!(
            Date::parse_from_json(Some(Value::String("2015-09-18".to_string()))).unwrap(),
            civil::date(2015, 9, 18)
        );
    }

    #[test]
    fn civil_time() {
        let time = civil::time(23, 56, 4, 0);
        let value = time.to_json();
        assert_eq!(value, Some(Value::String("23:56:04".to_string())));
        assert_eq!(
            Time::parse_from_json(Some(Value::String("23:56:04".to_string()))).unwrap(),
            civil::time(23, 56, 4, 0)
        );
    }

    fn assert_type<T>(value: T, format: &'static str)
    where
        T: Type<RawValueType = T, RawElementValueType = T> + Debug + PartialEq,
    {
        assert!(T::IS_REQUIRED);
        assert_eq!(T::name(), format!("string_{format}"));
        assert_eq!(
            T::schema_ref().unwrap_inline(),
            &MetaSchema::new_with_format("string", format)
        );
        assert_eq!(value.as_raw_value(), Some(&value));
        assert_eq!(value.raw_element_iter().collect::<Vec<_>>(), [&value]);
    }

    #[test]
    fn schemas_and_raw_values() {
        assert_type(Timestamp::UNIX_EPOCH, "date-time");
        assert_type(Zoned::UNIX_EPOCH, "zoned-date-time");
        assert_type(
            civil::date(2024, 2, 29).at(12, 30, 45, 0),
            "naive-date-time",
        );
        assert_type(civil::date(2024, 2, 29), "naive-date");
        assert_type(civil::time(12, 30, 45, 0), "naive-time");
    }

    async fn assert_invalid_input<T>()
    where
        T: ParseFromJSON + ParseFromParameter + ParseFromMultipartField + Debug,
    {
        assert_eq!(
            T::parse_from_json(None).unwrap_err().message(),
            ParseError::<T>::expected_input().message()
        );
        for value in [Value::Null, json!(true), json!(42), json!([]), json!({})] {
            assert_eq!(
                T::parse_from_json(Some(value.clone()))
                    .unwrap_err()
                    .message(),
                ParseError::<T>::expected_type(value).message()
            );
        }
        for value in ["", "not-a-date", "2024-02-30T25:61:61"] {
            assert!(T::parse_from_json(Some(json!(value))).is_err());
            assert!(T::parse_from_parameter(value).is_err());
        }
        assert_eq!(
            T::parse_from_multipart(None).await.unwrap_err().message(),
            ParseError::<T>::expected_input().message()
        );
    }

    #[tokio::test]
    async fn invalid_input() {
        assert_invalid_input::<Timestamp>().await;
        assert_invalid_input::<Zoned>().await;
        assert_invalid_input::<DateTime>().await;
        assert_invalid_input::<Date>().await;
        assert_invalid_input::<Time>().await;
    }

    fn assert_roundtrip<T>(value: T)
    where
        T: ParseFromJSON + ParseFromParameter + ToJSON + Debug + PartialEq,
    {
        let json = value.to_json().unwrap();
        assert_eq!(T::parse_from_json(Some(json.clone())).unwrap(), value);
        assert_eq!(
            T::parse_from_parameter(json.as_str().unwrap()).unwrap(),
            value
        );
    }

    #[test]
    fn subsecond_precision_and_bounds() {
        for nanosecond in [1, 123_456_789, 999_999_999] {
            let datetime = civil::date(2024, 2, 29).at(23, 59, 59, nanosecond);
            assert_roundtrip(datetime);
            assert_roundtrip(datetime.time());
            assert_roundtrip(
                datetime
                    .to_zoned(jiff::tz::TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
            );
        }
        assert_roundtrip(Timestamp::MIN);
        assert_roundtrip(Timestamp::MAX);
        assert_roundtrip(DateTime::MIN);
        assert_roundtrip(DateTime::MAX);
        assert_roundtrip(Date::MIN);
        assert_roundtrip(Date::MAX);
        assert_roundtrip(Time::MIN);
        assert_roundtrip(Time::MAX);
    }

    #[test]
    fn timestamp_normalizes_offset_to_utc() {
        let timestamp =
            Timestamp::parse_from_parameter("2024-02-29T12:30:45.123456789+05:30").unwrap();
        assert_eq!(
            timestamp.to_json(),
            Some(json!("2024-02-29T07:00:45.123456789Z"))
        );
        assert_roundtrip(timestamp);
    }

    #[test]
    fn zoned_preserves_zone_and_ambiguous_instants() {
        let inputs = [
            "2024-11-03T01:30:00.123456789-04:00[America/New_York]",
            "2024-11-03T01:30:00.123456789-05:00[America/New_York]",
            "2024-02-29T12:30:45.123456789+05:30[+05:30]",
        ];
        for input in inputs {
            let original = Zoned::parse_from_parameter(input).unwrap();
            let json = original.to_json();
            assert_eq!(json, Some(json!(input)));
            let parsed = Zoned::parse_from_json(json).unwrap();
            // Zoned equality compares instants, so check the zone separately.
            assert_eq!(parsed.timestamp(), original.timestamp());
            assert_eq!(parsed.datetime(), original.datetime());
            assert_eq!(parsed.offset(), original.offset());
            assert_eq!(parsed.time_zone(), original.time_zone());
        }
        let first = Zoned::parse_from_parameter(inputs[0]).unwrap();
        let second = Zoned::parse_from_parameter(inputs[1]).unwrap();
        assert_ne!(first.timestamp(), second.timestamp());
    }

    #[test]
    fn reject_missing_offset_or_zone_and_conflicting_offset() {
        assert!(Timestamp::parse_from_parameter("2024-02-29T12:30:45").is_err());
        assert!(Zoned::parse_from_parameter("2024-02-29T12:30:45Z").is_err());
        assert!(
            Zoned::parse_from_parameter("2024-07-01T12:30:45-05:00[America/New_York]").is_err()
        );
    }

    #[test]
    fn zoned_follows_native_unnamed_zone_formatting() {
        let datetime = civil::date(2024, 7, 1).at(12, 0, 0, 0);
        let posix = jiff::tz::TimeZone::posix("EST5EDT,M3.2.0,M11.1.0").unwrap();
        let original = datetime.to_zoned(posix).unwrap();
        assert_eq!(
            original.to_json(),
            Some(json!("2024-07-01T12:00:00-04:00[-04:00]"))
        );
        let parsed = Zoned::parse_from_json(original.to_json()).unwrap();
        assert_eq!(parsed.timestamp(), original.timestamp());
        assert_ne!(parsed.time_zone(), original.time_zone());

        // Jiff's native format rounds fixed offsets to minutes. Preserve that
        // behavior rather than claiming arbitrary Zoned values round-trip.
        let offset = jiff::tz::Offset::from_seconds(3_601).unwrap();
        let original = datetime
            .to_zoned(jiff::tz::TimeZone::fixed(offset))
            .unwrap();
        assert_eq!(
            original.to_json(),
            Some(json!("2024-07-01T12:00:00+01:00[+01:00]"))
        );
        let parsed = Zoned::parse_from_json(original.to_json()).unwrap();
        assert_eq!(parsed.datetime(), original.datetime());
        assert_ne!(parsed.timestamp(), original.timestamp());
    }
}
