use std::ops::{Deref, DerefMut};

use poem::{Request, RequestBody, Result};

use crate::{
    base::UrlQuery,
    error::ParseParamError,
    registry::{MetaParamIn, MetaSchemaRef, Registry},
    types::ParseFromParameter,
    ApiExtractor, ApiExtractorType, ExtractParamOptions,
};

/// Represents the parameters passed by the query string.
pub struct Query<T>(pub T);

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Query<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T: ParseFromParameter> ApiExtractor<'a> for Query<T> {
    const TYPES: &'static [ApiExtractorType] = &[ApiExtractorType::Parameter];
    const PARAM_IS_REQUIRED: bool = T::IS_REQUIRED;

    type ParamType = T;
    type ParamRawType = T::RawValueType;

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn param_in() -> Option<MetaParamIn> {
        Some(MetaParamIn::Query)
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
        let mut values = match request.extensions().get::<UrlQuery>() {
            Some(query) => query.get_all(param_opts.name),
            None => {
                return Err(ParseParamError {
                    name: param_opts.name,
                    reason: "UrlQuery service not found".to_string(),
                })?
            }
        }
        .peekable();

        match &param_opts.default_value {
            Some(default_value) if values.peek().is_none() => {
                return Ok(Self(default_value()));
            }
            _ => {}
        }

        if param_opts.explode {
            ParseFromParameter::parse_from_parameters(values)
                .map(Self)
                .map_err(|err| {
                    ParseParamError {
                        name: param_opts.name,
                        reason: err.into_message(),
                    }
                    .into()
                })
        } else {
            let first_value = match values.next() {
                Some(val) => val,
                None => {
                    return Err(ParseParamError {
                        name: param_opts.name,
                        reason: "parameter not found".to_string(),
                    }
                    .into())
                }
            };

            let values = first_value.split(',').map(|v| v.trim());
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
}
