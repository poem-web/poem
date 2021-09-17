use darling::util::SpannedValue;
use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;
use syn::Error;

use crate::{
    common_args::{MaximumValidator, MinimumValidator},
    error::GeneratorResult,
};

pub(crate) struct Validators<'a> {
    pub(crate) multiple_of: &'a Option<SpannedValue<f64>>,
    pub(crate) maximum: &'a Option<SpannedValue<MaximumValidator>>,
    pub(crate) minimum: &'a Option<SpannedValue<MinimumValidator>>,
    pub(crate) max_length: &'a Option<SpannedValue<usize>>,
    pub(crate) min_length: &'a Option<SpannedValue<usize>>,
    pub(crate) pattern: &'a Option<SpannedValue<String>>,
    pub(crate) max_items: &'a Option<SpannedValue<usize>>,
    pub(crate) min_items: &'a Option<SpannedValue<usize>>,
    pub(crate) unique_items: &'a bool,
}

impl<'a> Validators<'a> {
    fn create_validators(&self, crate_name: &TokenStream) -> GeneratorResult<Vec<TokenStream>> {
        let mut validators = Vec::new();

        if let Some(value) = self.multiple_of {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.1
            if **value <= 0.0 {
                return Err(Error::new(
                    value.span(),
                    "The value of `multipleOf` MUST be a number, strictly greater than 0.",
                )
                .into());
            }
            let value = &**value;
            validators.push(quote!(#crate_name::validation::MultipleOf::new(#value)));
        }

        if let Some(MaximumValidator { value, exclusive }) = self.maximum.as_deref() {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.2
            validators.push(quote!(#crate_name::validation::Maximum::new(#value, #exclusive)));
        }

        if let Some(MinimumValidator { value, exclusive }) = self.minimum.as_deref() {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.4
            validators.push(quote!(#crate_name::validation::Minimum::new(#value, #exclusive)));
        }

        if let Some(value) = self.max_length {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.6
            let value = &**value;
            validators.push(quote!(#crate_name::validation::MaxLength::new(#value)));
        }

        if let Some(value) = self.min_length {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.7
            let value = &**value;
            validators.push(quote!(#crate_name::validation::MinLength::new(#value)));
        }

        if let Some(value) = &self.pattern {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.8
            if let Err(err) = Regex::new(&**value) {
                return Err(Error::new(
                    value.span(),
                    format!("Invalid regular expression. {}", err),
                )
                .into());
            }
            let value = &**value;
            validators.push(quote!(#crate_name::validation::Pattern::new(#value)));
        }

        if let Some(value) = self.max_items {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.10
            let value = &**value;
            validators.push(quote!(#crate_name::validation::MaxItems::new(#value)));
        }

        if let Some(value) = self.min_items {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.11
            let value = &**value;
            validators.push(quote!(#crate_name::validation::MinItems::new(#value)));
        }

        if *self.unique_items {
            // https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-00#section-5.12
            validators.push(quote!(#crate_name::validation::UniqueItems::new()));
        }

        Ok(validators)
    }

    pub(crate) fn create_obj_field_checker(
        &self,
        crate_name: &TokenStream,
        field_name: &str,
    ) -> GeneratorResult<Option<TokenStream>> {
        let validators = self.create_validators(crate_name)?;
        if !validators.is_empty() {
            Ok(Some(quote! {
                for validator in [#(#validators),*] {
                    if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_value(&value) {
                        if !#crate_name::validation::Validator::check(&validator, value) {
                            return Err(#crate_name::types::ParseError::<Self>::custom(format!("field `{}` verification failed. {}", #field_name, validator)));
                        }
                    }
                }
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn create_param_checker(
        &self,
        crate_name: &TokenStream,
        arg_name: &str,
    ) -> GeneratorResult<Option<TokenStream>> {
        let validators = self.create_validators(crate_name)?;
        if !validators.is_empty() {
            Ok(Some(quote! {
                for validator in [#(#validators),*] {
                    if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_value(&value) {
                        if !#crate_name::validation::Validator::check(&validator, value) {
                            let err = #crate_name::ParseRequestError::ParseParam {
                                name: #arg_name,
                                reason: ::std::format!("verification failed. {}", validator),
                            };
                            return Err(#crate_name::poem::Error::bad_request(err));
                        }
                    }
                }
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn create_multipart_field_checker(
        &self,
        crate_name: &TokenStream,
        field_name: &str,
    ) -> GeneratorResult<Option<TokenStream>> {
        let validators = self.create_validators(crate_name)?;
        if !validators.is_empty() {
            Ok(Some(quote! {
                for validator in [#(#validators),*] {
                    if let ::std::option::Option::Some(value) = #crate_name::types::Type::as_value(&value) {
                        if !#crate_name::validation::Validator::check(&validator, value) {
                            return Err(#crate_name::ParseRequestError::ParseRequestBody {
                                reason: ::std::format!("field `{}` verification failed. {}", #field_name, validator),
                            });
                        }
                    }
                }
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn create_update_meta(
        &self,
        crate_name: &TokenStream,
    ) -> GeneratorResult<Option<TokenStream>> {
        let validators = self.create_validators(crate_name)?;
        if !validators.is_empty() {
            Ok(Some(quote! {
                for validator in [#(#validators),*] {
                    #crate_name::validation::ValidatorMeta::update_meta(&validator, schema);
                }
            }))
        } else {
            Ok(None)
        }
    }
}

pub(crate) trait HasValidators {
    fn validators(&self) -> Validators<'_>;
}

macro_rules! impl_has_validators {
    ($ty:ty) => {
        impl HasValidators for $ty {
            fn validators(&self) -> crate::validators::Validators<'_> {
                crate::validators::Validators {
                    multiple_of: &self.multiple_of,
                    maximum: &self.maximum,
                    minimum: &self.minimum,
                    max_length: &self.max_length,
                    min_length: &self.min_length,
                    pattern: &self.pattern,
                    max_items: &self.max_items,
                    min_items: &self.min_items,
                    unique_items: &self.unique_items,
                }
            }
        }
    };
}
