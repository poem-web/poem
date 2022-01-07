use std::str::FromStr;

use darling::{
    ast::{Data, Fields},
    util::{Ignored, SpannedValue},
    FromDeriveInput, FromVariant,
};
use mime::Mime;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Attribute, DeriveInput, Error, Generics, Type};

use crate::{
    error::GeneratorResult,
    utils::{get_crate_name, get_description, optional_literal},
};

#[derive(FromVariant)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ContentItem {
    ident: Ident,
    fields: Fields<Type>,

    #[darling(default)]
    content_type: Option<SpannedValue<String>>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai))]
struct ResponseContentArgs {
    ident: Ident,
    generics: Generics,
    data: Data<ContentItem, Ignored>,

    #[darling(default)]
    internal: bool,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: ResponseContentArgs = ResponseContentArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();
    let ident = &args.ident;
    let e = match &args.data {
        Data::Enum(e) => e,
        _ => {
            return Err(Error::new_spanned(
                ident,
                "ResponseContent can only be applied to an enum.",
            )
            .into())
        }
    };

    let mut into_responses = Vec::new();
    let mut media_types = Vec::new();
    let mut schemas = Vec::new();

    for (idx, variant) in e.iter().enumerate() {
        let item_ident = &variant.ident;

        match variant.fields.len() {
            1 => {
                // Item(payload)
                let item_ty = &variant.fields.fields[0];
                let content_type = match &variant.content_type {
                    Some(content_type) => {}
                };
                media_types.push(quote! {
                    #crate_name::registry::MetaMediaType {
                        content_type: #content_type,
                    }
                });
                into_responses.push(quote! {});
                content.push(quote! {
                    #crate_name::registry::MetaMediaType {
                        content_type: #content_type,
                        schema: <#payload_ty as #crate_name::payload::Payload>::schema_ref(),
                    }
                });
                schemas.push(payload_ty);
            }
            _ => {
                return Err(
                    Error::new_spanned(&variant.ident, "Incorrect request definition.").into(),
                )
            }
        }
    }

    let expanded = {
        quote! {
            #[#crate_name::__private::poem::async_trait]
            impl #impl_generics #crate_name::ResponseContent for #ident #ty_generics #where_clause {
                fn media_types() -> Vec<#crate_name::registry::MetaMediaType> {
                    ::std::vec![#(#media_types),*]
                }

                fn register(registry: &mut #crate_name::registry::Registry) {
                    #(<#schemas as #crate_name::payload::Payload>::register(registry);)*
                }
            }
        }
    };

    Ok(expanded)
}
