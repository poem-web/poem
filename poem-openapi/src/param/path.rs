use std::ops::{Deref, DerefMut};

use poem::{Request, RequestBody};

use crate::{
    registry::{MetaParamIn, MetaSchemaRef, Registry},
    types::ParseFromParameter,
    ApiExtractor, ApiExtractorType, ExtractParamOptions, ParseRequestError,
};

/// Represents the parameters passed by the URI path.
pub struct Path<T>(pub T);

impl<T> Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Path<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[poem::async_trait]
impl<'a, T: ParseFromParameter> ApiExtractor<'a> for Path<T> {
    const TYPE: ApiExtractorType = ApiExtractorType::Parameter;
    const PARAM_IS_REQUIRED: bool = T::IS_REQUIRED;

    type ParamType = T;
    type ParamRawType = T::RawValueType;

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn param_in() -> Option<MetaParamIn> {
        Some(MetaParamIn::Path)
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
    ) -> Result<Self, ParseRequestError> {
        let value = match (
            request.raw_path_param(param_opts.name),
            &param_opts.default_value,
        ) {
            (Some(value), _) => Some(value),
            (None, Some(default_value)) => return Ok(Self(default_value())),
            (None, _) => None,
        };

        ParseFromParameter::parse_from_parameters(value)
            .map(Self)
            .map_err(|err| ParseRequestError::ParseParam {
                name: param_opts.name,
                reason: err.into_message(),
            })
    }
}
