use darling::{util::SpannedValue, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;
use syn::{Error, Expr, Type};

use crate::{
    common_args::{MaximumValidator, MinimumValidator},
    error::GeneratorResult,
};

struct ValidatorsTokenStream {
    container_validators: Vec<TokenStream>,
    elem_validators: Vec<TokenStream>,
    custom_validators: Vec<TokenStream>,
}

#[derive(FromMeta, Default, Clone)]
pub(crate) struct Validators {
    // for elements
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

    // for containers
    #[darling(default)]
    max_items: Option<SpannedValue<usize>>,
    #[darling(default)]
    min_items: Option<SpannedValue<usize>>,
    #[darling(default)]
    unique_items: Option<SpannedValue<bool>>,
    #[darling(default)]
    max_properties: Option<SpannedValue<usize>>,
    #[darling(default)]
    min_properties: Option<SpannedValue<usize>>,

    // custom validators for elements
    #[darling(default, multiple)]
    custom: Vec<SpannedValue<String>>,
}

impl Validators {
    fn create_validators(
        &self,
        crate_name: &TokenStream,
    ) -> GeneratorResult<ValidatorsTokenStream> {
        let mut container_validators = Vec::new();
        let mut elem_validators = Vec::new();
        let mut custom_validators = Vec::new();

        //////////////////////////////////////////////////////////////////////////////
        // element validators
        //////////////////////////////////////////////////////////////////////////////

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
            if let Err(err) = Regex::new(value) {
                return Err(
                    Error::new(value.span(), format!("Invalid regular expression. {err}")).into(),
                );
            }
            let value = &**value;
            elem_validators.push(quote!(#crate_name::validation::Pattern::new(#value)));
        }

        //////////////////////////////////////////////////////////////////////////////
        // custom validators
        //////////////////////////////////////////////////////////////////////////////

        for custom in &self.custom {
            let create_custom_validator: Expr =
                syn::parse_str(custom).map_err(|err| Error::new(custom.span(), err.to_string()))?;
            custom_validators.push(quote!(#create_custom_validator));
        }

        //////////////////////////////////////////////////////////////////////////////
        // container validators
        //////////////////////////////////////////////////////////////////////////////

        if let Some(value) = self.max_items {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.10
            let value = &*value;
            container_validators.push(quote!(#crate_name::validation::MaxItems::new(#value)));
        }

        if let Some(value) = self.min_items {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.11
            let value = &*value;
            container_validators.push(quote!(#crate_name::validation::MinItems::new(#value)));
        }

        if self.unique_items.map(|value| *value).unwrap_or_default() {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.12
            container_validators.push(quote!(#crate_name::validation::UniqueItems::new()));
        }

        if let Some(value) = self.max_properties {
            // https://json-schema.org/draft/2020-12/json-schema-validation.html#rfc.section.6.5.1
            let value = &*value;
            container_validators.push(quote!(#crate_name::validation::MaxProperties::new(#value)));
        }

        if let Some(value) = self.min_properties {
            // https://json-schema.org/draft/2020-12/json-schema-validation.html#rfc.section.6.5.2
            let value = &*value;
            container_validators.push(quote!(#crate_name::validation::MinProperties::new(#value)));
        }

        Ok(ValidatorsTokenStream {
            container_validators,
            elem_validators,
            custom_validators,
        })
    }

    pub(crate) fn create_obj_field_checker(
        &self,
        crate_name: &TokenStream,
        field_name: &str,
    ) -> GeneratorResult<TokenStream> {
        let ValidatorsTokenStream {
            container_validators,
            elem_validators,
            custom_validators,
        } = self.create_validators(crate_name)?;
        let elem_validators = elem_validators.into_iter().chain(custom_validators);

        Ok(quote! {
            #(
            for elem in #crate_name::types::Type::raw_element_iter(&value) {
                let validator = #elem_validators;
                if !#crate_name::validation::Validator::check(&validator, elem) {
                    return Err(#crate_name::types::ParseError::<Self>::custom(format!("field `{}` verification failed. {}", #field_name, validator)));
                }
            }
            )*

            #(
            if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                let validator = #container_validators;
                if !#crate_name::validation::Validator::check(&validator, value) {
                    return Err(#crate_name::types::ParseError::<Self>::custom(format!("field `{}` verification failed. {}", #field_name, validator)));
                }
            }
            )*
        })
    }

    pub(crate) fn create_param_checker(
        &self,
        crate_name: &TokenStream,
        res_ty: &Type,
        arg_name: &str,
    ) -> GeneratorResult<Option<TokenStream>> {
        let ValidatorsTokenStream {
            container_validators,
            elem_validators,
            custom_validators,
        } = self.create_validators(crate_name)?;
        let elem_validators = elem_validators.into_iter().chain(custom_validators);

        Ok(Some(quote! {
            #(
            if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                let validator = #container_validators;
                if !#crate_name::validation::Validator::check(&validator, value) {
                    let err = #crate_name::error::ParseParamError {
                        name: #arg_name,
                        reason: ::std::format!("verification failed. {}", validator),
                    };

                    if <#res_ty as #crate_name::ApiResponse>::BAD_REQUEST_HANDLER {
                        let res = <#res_ty as #crate_name::ApiResponse>::from_parse_request_error(std::convert::Into::into(err));
                        let res = #crate_name::__private::poem::error::IntoResult::into_result(res);
                        return ::std::result::Result::map(res, #crate_name::__private::poem::IntoResponse::into_response);
                    } else {
                        return ::std::result::Result::Err(std::convert::Into::into(err));
                    }
                }
            }
            )*

            #(
            if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                let validator = #elem_validators;
                if !#crate_name::validation::Validator::check(&validator, value) {
                    let err = #crate_name::error::ParseParamError {
                        name: #arg_name,
                        reason: ::std::format!("verification failed. {}", validator),
                    };

                    if <#res_ty as #crate_name::ApiResponse>::BAD_REQUEST_HANDLER {
                        let res = <#res_ty as #crate_name::ApiResponse>::from_parse_request_error(std::convert::Into::into(err));
                        let res = #crate_name::__private::poem::error::IntoResult::into_result(res);
                        return ::std::result::Result::map(res, #crate_name::__private::poem::IntoResponse::into_response);
                    } else {
                        return ::std::result::Result::Err(std::convert::Into::into(err));
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
    ) -> GeneratorResult<TokenStream> {
        let ValidatorsTokenStream {
            container_validators,
            elem_validators,
            custom_validators,
        } = self.create_validators(crate_name)?;
        let elem_validators = elem_validators.into_iter().chain(custom_validators);

        Ok(quote! {
            #(
            for item in #crate_name::types::Type::raw_element_iter(&value) {
                let validator = #elem_validators;
                if !#crate_name::validation::Validator::check(&validator, item) {
                    return Err(::std::convert::Into::into(#crate_name::error::ParseMultipartError {
                        reason: ::std::format!("field `{}` verification failed. {}", #field_name, validator),
                    }));
                }
            }
            )*

            #(
            if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_raw_value(&value) {
                let validator = #container_validators;
                if !#crate_name::validation::Validator::check(&validator, value) {
                    return Err(::std::convert::Into::into(#crate_name::error::ParseMultipartError {
                        reason: ::std::format!("field `{}` verification failed. {}", #field_name, validator),
                    }));
                }
            }
            )*
        })
    }

    pub(crate) fn create_update_meta(
        &self,
        crate_name: &TokenStream,
    ) -> GeneratorResult<TokenStream> {
        let ValidatorsTokenStream {
            container_validators,
            elem_validators,
            ..
        } = self.create_validators(crate_name)?;

        let update_elem_meta = quote! {
            if original_schema.is_array() {
                let mut items_schema = #crate_name::registry::MetaSchema::ANY;
                #(
                #crate_name::validation::ValidatorMeta::update_meta(&#elem_validators, &mut items_schema);
                )*
                schema.items = ::std::option::Option::Some(::std::boxed::Box::new(#crate_name::registry::MetaSchemaRef::Inline(::std::boxed::Box::new(items_schema))));
            } else if original_schema.is_object() {
                let mut additional_properties_schema = #crate_name::registry::MetaSchema::ANY;
                #(
                #crate_name::validation::ValidatorMeta::update_meta(&#elem_validators, &mut additional_properties_schema);
                )*
                schema.additional_properties = ::std::option::Option::Some(::std::boxed::Box::new(#crate_name::registry::MetaSchemaRef::Inline(::std::boxed::Box::new(additional_properties_schema))));
            } else {
                #(
                #crate_name::validation::ValidatorMeta::update_meta(&#elem_validators, &mut schema);
                )*
            }
        };
        let update_elem_meta = if !elem_validators.is_empty() {
            Some(update_elem_meta)
        } else {
            None
        };

        Ok(quote! {
            #(
            #crate_name::validation::ValidatorMeta::update_meta(&#container_validators, &mut schema);
            )*
            #update_elem_meta
        })
    }
}
