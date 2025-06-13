use std::ops::{Deref, DerefMut};

use itertools::Either;
use poem::{Request, RequestBody, Result};

use crate::{
    ApiExtractor, ApiExtractorType, ExtractParamOptions,
    error::ParseParamError,
    registry::{MetaParamIn, MetaSchemaRef, Registry},
    types::ParseFromParameter,
};

/// Represents the parameters passed by the request header.
pub struct Header<T>(pub T);

impl<T> Deref for Header<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Header<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T: ParseFromParameter> ApiExtractor<'a> for Header<T> {
    const TYPES: &'static [ApiExtractorType] = &[ApiExtractorType::Parameter];
    const PARAM_IS_REQUIRED: bool = T::IS_REQUIRED;

    type ParamType = T;
    type ParamRawType = T::RawValueType;

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn param_in() -> Option<MetaParamIn> {
        Some(MetaParamIn::Header)
    }

    fn param_schema_ref() -> Option<MetaSchemaRef> {
        Some(T::schema_ref())
    }

    fn param_raw_type(&self) -> Option<&Self::ParamRawType> {
        self.0.as_raw_value()
    }

    async fn from_request(
        request: &'a Request,
        _body: &mut RequestBody,
        param_opts: ExtractParamOptions<Self::ParamType>,
    ) -> Result<Self> {
        let mut values = if !param_opts.ignore_case {
            Either::Left(
                request
                    .headers()
                    .get_all(param_opts.name)
                    .iter()
                    .filter_map(|value| value.to_str().ok()),
            )
        } else {
            Either::Right(
                request
                    .headers()
                    .iter()
                    .filter(|(n, _)| n.as_str().eq_ignore_ascii_case(param_opts.name))
                    .filter_map(|(_, value)| value.to_str().ok()),
            )
        }
        .peekable();

        match &param_opts.default_value {
            Some(default_value) if values.peek().is_none() => {
                return Ok(Self(default_value()));
            }
            _ => {}
        }

        ParseFromParameter::parse_from_parameters(values)
            .map(Self)
            .map_err(|err| {
                ParseParamError {
                    name: param_opts.name,
                    reason: err.into_message(),
                }
                .into()
            })
    }
}
