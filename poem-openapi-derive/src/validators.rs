use darling::{util::SpannedValue, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;
use syn::{Error, Type};

use crate::{
    common_args::{MaximumValidator, MinimumValidator},
    error::GeneratorResult,
};

#[derive(FromMeta, Default, Clone)]
pub(crate) struct Validators {
    #[darling(default)]
    multiple_of: Option<SpannedValue<f64>>,
    #[darling(default)]
    maximum: Option<SpannedValue<MaximumValidator>>,
    #[darling(default)]
    minimum: Option<SpannedValue<MinimumValidator>>,
    #[darling(default)]
    max_length: Option<SpannedValue<usize>>,
    #[darling(default)]
    min_length: Option<SpannedValue<usize>>,
    #[darling(default)]
    pattern: Option<SpannedValue<String>>,
    #[darling(default)]
    max_items: Option<SpannedValue<usize>>,
    #[darling(default)]
    min_items: Option<SpannedValue<usize>>,
    #[darling(default)]
    unique_items: bool,
    #[darling(default)]
    list: SpannedValue<bool>,
}

impl Validators {
    fn create_validators(
        &self,
        crate_name: &TokenStream,
    ) -> GeneratorResult<(Vec<TokenStream>, Vec<TokenStream>)> {
        let mut list_validators = Vec::new();
        let mut elem_validators = Vec::new();

        if let Some(value) = self.multiple_of {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.1
            if *value <= 0.0 {
                return Err(Error::new(
                    value.span(),
                    "The value of `multipleOf` MUST be a number, strictly greater than 0.",
                )
                .into());
            }
            let value = &*value;
            elem_validators.push(quote!(#crate_name::validation::MultipleOf::new(#value)));
        }

        if let Some(MaximumValidator { value, exclusive }) = self.maximum.as_deref() {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.2
            elem_validators.push(quote!(#crate_name::validation::Maximum::new(#value, #exclusive)));
        }

        if let Some(MinimumValidator { value, exclusive }) = self.minimum.as_deref() {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.4
            elem_validators.push(quote!(#crate_name::validation::Minimum::new(#value, #exclusive)));
        }

        if let Some(value) = self.max_length {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.6
            let value = &*value;
            elem_validators.push(quote!(#crate_name::validation::MaxLength::new(#value)));
        }

        if let Some(value) = self.min_length {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.7
            let value = &*value;
            elem_validators.push(quote!(#crate_name::validation::MinLength::new(#value)));
        }

        if let Some(value) = &self.pattern {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.8
            if let Err(err) = Regex::new(&*value) {
                return Err(Error::new(
                    value.span(),
                    format!("Invalid regular expression. {}", err),
                )
                .into());
            }
            let value = &**value;
            elem_validators.push(quote!(#crate_name::validation::Pattern::new(#value)));
        }

        if let Some(value) = self.max_items {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.10
            let value = &*value;
            list_validators.push(quote!(#crate_name::validation::MaxItems::new(#value)));
        }

        if let Some(value) = self.min_items {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.11
            let value = &*value;
            list_validators.push(quote!(#crate_name::validation::MinItems::new(#value)));
        }

        if self.unique_items {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.12
            list_validators.push(quote!(#crate_name::validation::UniqueItems::new()));
        }

        Ok((list_validators, elem_validators))
    }

    pub(crate) fn create_obj_field_checker(
        &self,
        crate_name: &TokenStream,
        field_name: &str,
    ) -> GeneratorResult<Option<TokenStream>> {
        let (list_validators, elem_validators) = self.create_validators(crate_name)?;
        if !*self.list {
            let validators = list_validators.into_iter().chain(elem_validators);
            Ok(Some(quote! {
                #(
                    if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                        let validator = #validators;
                        if !#crate_name::validation::Validator::check(&validator, value) {
                            return Err(#crate_name::types::ParseError::<Self>::custom(format!("field `{}` verification failed. {}", #field_name, validator)));
                        }
                    }
                )*
            }))
        } else {
            Ok(Some(quote! {
                #(
                    if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                        let validator = #list_validators;
                        if !#crate_name::validation::Validator::check(&validator, value) {
                            return Err(#crate_name::types::ParseError::<Self>::custom(format!("field `{}` verification failed. {}", #field_name, validator)));
                        }
                    }
                )*
                #(
                    if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                        for item in value {
                            if let ::std::option::Option::Some(item) = #crate_name::types::Type::as_raw_value(item) {
                                let validator = #elem_validators;
                                if !#crate_name::validation::Validator::check(&validator, item) {
                                    return Err(#crate_name::types::ParseError::<Self>::custom(format!("field `{}` verification failed. {}", #field_name, validator)));
                                }
                            }
                        }
                    }
                )*
            }))
        }
    }

    pub(crate) fn create_param_checker(
        &self,
        crate_name: &TokenStream,
        res_ty: &Type,
        arg_name: &str,
    ) -> GeneratorResult<Option<TokenStream>> {
        let (list_validators, elem_validators) = self.create_validators(crate_name)?;
        if *self.list {
            return Err(Error::new(
                self.list.span(),
                "The `list` attribute is not supported for parameters.",
            )
            .into());
        }

        let validators = list_validators.into_iter().chain(elem_validators);
        Ok(Some(quote! {
            #(
                if let ::std::option::Option::Some(item) = #crate_name::types::Type::as_raw_value(value) {
                    let validator = #validators;
                    if !#crate_name::validation::Validator::check(&validator, value) {
                        let err = #crate_name::ParseRequestError::ParseParam {
                            name: #arg_name,
                            reason: ::std::format!("verification failed. {}", validator),
                        };
                        if <#res_ty as #crate_name::ApiResponse>::BAD_REQUEST_HANDLER {
                            let resp = <#res_ty as #crate_name::ApiResponse>::from_parse_request_error(err);
                            return #crate_name::__private::poem::IntoResponse::into_response(resp);
                        } else {
                            return #crate_name::__private::poem::IntoResponse::into_response(err);
                        }
                    }
                }
            )*
        }))
    }

    pub(crate) fn create_multipart_field_checker(
        &self,
        crate_name: &TokenStream,
        field_name: &str,
    ) -> GeneratorResult<Option<TokenStream>> {
        let (list_validators, elem_validators) = self.create_validators(crate_name)?;
        if !*self.list {
            let validators = list_validators.into_iter().chain(elem_validators);
            Ok(Some(quote! {
                #(
                    if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                        let validator = #validators;
                        if !#crate_name::validation::Validator::check(&validator, value) {
                            return Err(#crate_name::ParseRequestError::ParseRequestBody(
                                #crate_name::__private::poem::Response::builder()
                                    .status(#crate_name::__private::poem::http::StatusCode::BAD_REQUEST)
                                    .body(::std::format!("field `{}` verification failed. {}", #field_name, validator))
                            ));
                        }
                    }
                )*
            }))
        } else {
            Ok(Some(quote! {
                #(
                    if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                        let validator = #list_validators;
                        if !#crate_name::validation::Validator::check(&validator, value) {
                            return Err(#crate_name::ParseRequestError::ParseRequestBody(
                                #crate_name::__private::poem::Response::builder()
                                    .status(#crate_name::__private::poem::http::StatusCode::BAD_REQUEST)
                                    .body(::std::format!("field `{}` verification failed. {}", #field_name, validator))
                            ));
                        }
                    }
                )*
                #(
                    if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                        let validator = #elem_validators;
                        for item in value {
                            if let ::std::option::Option::Some(item) = #crate_name::types::Type::as_raw_value(item) {
                                if !#crate_name::validation::Validator::check(&validator, item) {
                                    return Err(#crate_name::ParseRequestError::ParseRequestBody(
                                        #crate_name::__private::poem::Response::builder()
                                            .status(#crate_name::__private::poem::http::StatusCode::BAD_REQUEST)
                                            .body(::std::format!("field `{}` verification failed. {}", #field_name, validator))
                                    ));
                                }
                            }
                        }
                    }
                )*
            }))
        }
    }

    pub(crate) fn create_update_meta(
        &self,
        crate_name: &TokenStream,
    ) -> GeneratorResult<Option<TokenStream>> {
        let (list_validators, elem_validators) = self.create_validators(crate_name)?;
        if !*self.list {
            let validators = list_validators.into_iter().chain(elem_validators);
            Ok(Some(quote! {
                #(
                    #crate_name::validation::ValidatorMeta::update_meta(&#validators, &mut schema);
                )*
            }))
        } else {
            Ok(Some(quote! {
                #(
                    #crate_name::validation::ValidatorMeta::update_meta(&#list_validators, &mut schema);
                )*
                #(
                    let mut items_schema = #crate_name::registry::MetaSchema::ANY;
                    #crate_name::validation::ValidatorMeta::update_meta(&#elem_validators, &mut items_schema);
                    schema.items = ::std::option::Option::Some(::std::boxed::Box::new(#crate_name::registry::MetaSchemaRef::Inline(::std::boxed::Box::new(items_schema))));
                )*
            }))
        }
    }
}
