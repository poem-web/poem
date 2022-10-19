#[macro_export]
/// This macro implements ApiExtractor for your type, with additional bounds
/// if you want to.
macro_rules! impl_apirequest_for_payload {
    ($ty:ty) => {
        impl_apirequest_for_payload!($ty,);
    };

    ($ty:ty, $($bounds:tt)*) => {
        #[poem::async_trait]
        impl<'a, $($bounds)*> $crate::ApiExtractor<'a> for $ty {
            const TYPE: $crate::ApiExtractorType = $crate::ApiExtractorType::RequestObject;

            type ParamType = ();
            type ParamRawType = ();

            fn register(registry: &mut $crate::registry::Registry) {
                <Self as $crate::payload::Payload>::register(registry);
            }

            fn request_meta() -> Option<$crate::registry::MetaRequest> {
                Some($crate::registry::MetaRequest {
                    description: None,
                    content: vec![$crate::registry::MetaMediaType {
                        content_type: <Self as $crate::payload::Payload>::CONTENT_TYPE,
                        schema: <Self as $crate::payload::Payload>::schema_ref(),
                    }],
                    required: <Self as $crate::payload::ParsePayload>::IS_REQUIRED,
                })
            }

            async fn from_request(
                request: &'a poem::Request,
                body: &mut poem::RequestBody,
                _param_opts: $crate::ExtractParamOptions<Self::ParamType>,
            ) -> poem::Result<Self> {
                match request.content_type() {
                    Some(content_type) => {
                        if <$ty>::check_content_type(content_type) {
                            <Self as $crate::payload::ParsePayload>::from_request(request, body).await
                        } else {
                            return Err($crate::error::ContentTypeError::NotSupported {
                                content_type: content_type.to_string(),
                            }.into());

                        }
                    }
                    None => Err($crate::error::ContentTypeError::ExpectContentType.into()),
                }
            }
        }
    };
}
