use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use poem::{http::HeaderValue, web::Field};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

macro_rules! meta_scheme {
    ($format:literal,) => {
        MetaSchema::new_with_format("string", $format)
    };
    ($format:literal, $($oneof:ty),+) => {
        MetaSchema {
            one_of: vec![$(<$oneof as Type>::schema_ref()),+],
            ..MetaSchema::ANY
        }
    };
}

macro_rules! impl_type_for_ip {
    ($(($ty:ty, $format:literal $(,)? $($oneof:ty),*)),*) => {
        $(
        impl Type for $ty {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> Cow<'static, str> {
                format!("string({})", $format).into()
            }

            fn schema_ref() -> MetaSchemaRef {
                MetaSchemaRef::Inline(Box::new(meta_scheme!($format, $($oneof),*)))
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
                let value = value.unwrap_or_default();
                if let Value::String(value) = value {
                    Ok(value.parse()?)
                } else {
                    Err(ParseError::expected_type(value))
                }
            }
        }

        impl ParseFromParameter for $ty {
            fn parse_from_parameter(value: &str) -> ParseResult<Self> {
                value.parse().map_err(ParseError::custom)
            }
        }

        #[poem::async_trait]
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

        impl ToHeader for $ty {
            fn to_header(&self) -> Option<HeaderValue> {
                HeaderValue::from_str(&self.to_string()).ok()
            }
        }
        )*
    }
}

impl_type_for_ip!(
    (Ipv4Addr, "ipv4"),
    (Ipv6Addr, "ipv6"),
    (IpAddr, "ip", Ipv4Addr, Ipv6Addr)
);

#[cfg(feature = "ipnet")]
impl_type_for_ip!(
    (ipnet::Ipv4Net, "ipv4net"),
    (ipnet::Ipv6Net, "ipv6net"),
    (ipnet::IpNet, "ipnet", ipnet::Ipv4Net, ipnet::Ipv6Net)
);
