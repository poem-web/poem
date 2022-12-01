use darling::{
    ast::{Data, Style},
    util::Ignored,
    FromDeriveInput,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Attribute, DeriveInput, Error, Generics, Type};

use crate::{
    common_args::ExternalDocument,
    error::GeneratorResult,
    utils::{
        get_crate_name, get_summary_and_description, optional_literal, optional_literal_string,
    },
};

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct NewTypeArgs {
    ident: Ident,
    generics: Generics,
    data: Data<Ignored, Type>,
    attrs: Vec<Attribute>,

    #[darling(default)]
    internal: bool,
    #[darling(default = "default_true")]
    from_json: bool,
    #[darling(default = "default_true")]
    from_parameter: bool,
    #[darling(default = "default_true")]
    from_multipart: bool,
    #[darling(default = "default_true")]
    to_json: bool,
    #[darling(default = "default_true")]
    to_header: bool,
    #[darling(default)]
    external_docs: Option<ExternalDocument>,
    #[darling(default)]
    example: bool,
}

const fn default_true() -> bool {
    true
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: NewTypeArgs = NewTypeArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();
    let ident = &args.ident;
    let (summary, description) = get_summary_and_description(&args.attrs)?;
    let summary = optional_literal_string(&summary);
    let description = optional_literal(&description);

    let fields = match &args.data {
        Data::Struct(e) => e,
        _ => {
            return Err(
                Error::new_spanned(ident, "NewType can only be applied to an struct.").into(),
            )
        }
    };

    if fields.style == Style::Tuple && fields.fields.len() != 1 {
        return Err(Error::new_spanned(
            ident,
            "Only one unnamed field is allowed in the SecurityScheme struct.",
        )
        .into());
    }

    let inner_ty = &fields.fields[0];
    let external_docs = match &args.external_docs {
        Some(external_docs) => {
            let s = external_docs.to_token_stream(&crate_name);
            quote!(::std::option::Option::Some(#s))
        }
        None => quote!(::std::option::Option::None),
    };
    let example = if args.example {
        quote! {
            {
                let value = <Self as #crate_name::types::Example>::example();
                <Self as #crate_name::types::ToJSON>::to_json(&value)
            }
        }
    } else {
        quote!(None)
    };

    let schema_ref = quote! {
        <#inner_ty as #crate_name::types::Type>::schema_ref().merge(#crate_name::registry::MetaSchema {
            title: #summary,
            description: #description,
            external_docs: #external_docs,
            example: #example,
            ..#crate_name::registry::MetaSchema::ANY
        })
    };

    let from_json = if args.from_json {
        Some(quote! {
            impl #impl_generics #crate_name::types::ParseFromJSON for #ident #ty_generics #where_clause {
                fn parse_from_json(value: ::std::option::Option<#crate_name::__private::serde_json::Value>) -> #crate_name::types::ParseResult<Self> {
                    let value = ::std::result::Result::map_err(<#inner_ty as #crate_name::types::ParseFromJSON>::parse_from_json(value), poem_openapi::types::ParseError::propagate)?;
                    ::std::result::Result::Ok(#ident(value))
                }
            }
        })
    } else {
        None
    };

    let from_parameter = if args.from_parameter {
        Some(quote! {
            impl #impl_generics #crate_name::types::ParseFromParameter for #ident #ty_generics #where_clause {
                fn parse_from_parameter(value: &str) -> #crate_name::types::ParseResult<Self> {
                    let value = ::std::result::Result::map_err(<#inner_ty as #crate_name::types::ParseFromParameter>::parse_from_parameter(value), poem_openapi::types::ParseError::propagate)?;
                    ::std::result::Result::Ok(#ident(value))
                }

                fn parse_from_parameters<I: ::std::iter::IntoIterator<Item = A>, A: ::std::convert::AsRef<str>>(
                    iter: I,
                ) -> #crate_name::types::ParseResult<Self> {
                    let value = ::std::result::Result::map_err(<#inner_ty as #crate_name::types::ParseFromParameter>::parse_from_parameters(iter), poem_openapi::types::ParseError::propagate)?;
                    ::std::result::Result::Ok(#ident(value))
                }
            }
        })
    } else {
        None
    };

    let from_multipart = if args.from_multipart {
        Some(quote! {
            #[#crate_name::__private::poem::async_trait]
            impl #impl_generics #crate_name::types::ParseFromMultipartField for #ident #ty_generics #where_clause {
                async fn parse_from_multipart(field: ::std::option::Option<#crate_name::__private::poem::web::Field>) -> #crate_name::types::ParseResult<Self> {
                    let value = ::std::result::Result::map_err(<#inner_ty as #crate_name::types::ParseFromMultipartField>::parse_from_multipart(field).await, poem_openapi::types::ParseError::propagate)?;
                    ::std::result::Result::Ok(#ident(value))
                }

                async fn parse_from_repeated_field(self, field: #crate_name::__private::poem::web::Field) -> #crate_name::types::ParseResult<Self> {
                    let value = ::std::result::Result::map_err(<#inner_ty as #crate_name::types::ParseFromMultipartField>::parse_from_repeated_field(self.0, field).await, poem_openapi::types::ParseError::propagate)?;
                    ::std::result::Result::Ok(#ident(value))
                }
            }
        })
    } else {
        None
    };

    let to_json = if args.to_json {
        Some(quote! {
            impl #impl_generics #crate_name::types::ToJSON for #ident #ty_generics #where_clause {
                fn to_json(&self) -> ::std::option::Option<#crate_name::__private::serde_json::Value> {
                    <#inner_ty as #crate_name::types::ToJSON>::to_json(&self.0)
                }
            }
        })
    } else {
        None
    };

    let to_header = if args.to_header {
        Some(quote! {
            impl #impl_generics #crate_name::types::ToHeader for #ident #ty_generics #where_clause {
                fn to_header(&self) -> Option<#crate_name::__private::poem::http::HeaderValue> {
                    <#inner_ty as #crate_name::types::ToHeader>::to_header(&self.0)
                }
            }
        })
    } else {
        None
    };

    let expanded = quote! {
        impl #impl_generics #crate_name::types::Type for #ident #ty_generics #where_clause {
            const IS_REQUIRED: bool = <#inner_ty as #crate_name::types::Type>::IS_REQUIRED;
            type RawValueType = <#inner_ty as #crate_name::types::Type>::RawValueType;
            type RawElementValueType = <#inner_ty as #crate_name::types::Type>::RawElementValueType;

            fn name() -> ::std::borrow::Cow<'static, str> {
                <#inner_ty as #crate_name::types::Type>::name()
            }

            fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                #schema_ref
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                <#inner_ty as #crate_name::types::Type>::register(registry);
            }

            fn as_raw_value(&self) -> ::std::option::Option<&Self::RawValueType> {
                <#inner_ty as #crate_name::types::Type>::as_raw_value(&self.0)
            }

            fn raw_element_iter<'a>(
                &'a self,
            ) -> ::std::boxed::Box<dyn ::std::iter::Iterator<Item = &'a Self::RawElementValueType> + 'a> {
                <#inner_ty as #crate_name::types::Type>::raw_element_iter(&self.0)
            }
        }

        #from_json
        #from_parameter
        #from_multipart
        #to_json
        #to_header
    };

    Ok(expanded)
}
