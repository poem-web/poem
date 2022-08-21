use darling::{
    ast::{Data, Fields},
    util::{Ignored, SpannedValue},
    FromDeriveInput, FromVariant,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DeriveInput, Error, Generics, Type};

use crate::{error::GeneratorResult, utils::get_crate_name};

#[derive(FromVariant)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ContentItem {
    ident: Ident,
    fields: Fields<Type>,

    #[darling(default)]
    content_type: Option<SpannedValue<String>>,
    #[darling(default)]
    actual_type: Option<Type>,
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
    let mut schemas = Vec::new();
    let mut content = Vec::new();

    for variant in e.iter() {
        let item_ident = &variant.ident;

        match variant.fields.len() {
            1 => {
                // Item(payload)
                let item_ty = &variant.fields.fields[0];

                let (content_type, schema_ref) = if let Some(content_type) = &variant.content_type {
                    let content_type = content_type.as_str();
                    (
                        quote!(#content_type),
                        quote!(<#item_ty as #crate_name::payload::Payload>::schema_ref()),
                    )
                } else if let Some(actual_type) = &variant.actual_type {
                    (
                        quote!(<#actual_type as #crate_name::payload::Payload>::CONTENT_TYPE),
                        quote!(<#actual_type as #crate_name::payload::Payload>::schema_ref()),
                    )
                } else {
                    (
                        quote!(<#item_ty as #crate_name::payload::Payload>::CONTENT_TYPE),
                        quote!(<#item_ty as #crate_name::payload::Payload>::schema_ref()),
                    )
                };

                let update_content_type = if let Some(content_type) = &variant.content_type {
                    let content_type = content_type.as_str();
                    quote! {
                        resp.headers_mut().insert(#crate_name::__private::poem::http::header::CONTENT_TYPE,
                            #crate_name::__private::poem::http::HeaderValue::from_static(#content_type));
                    }
                } else if let Some(actual_type) = &variant.actual_type {
                    quote! {
                        resp.headers_mut().insert(#crate_name::__private::poem::http::header::CONTENT_TYPE,
                            #crate_name::__private::poem::http::HeaderValue::from_static(<#actual_type as #crate_name::payload::Payload>::CONTENT_TYPE)
                        );
                    }
                } else {
                    quote!()
                };
                into_responses.push(quote! {
                    #ident::#item_ident(resp) => {
                        let mut resp = #crate_name::__private::poem::IntoResponse::into_response(resp);
                        #update_content_type
                        resp
                    },
                });
                content.push(quote! {
                    #crate_name::registry::MetaMediaType {
                        content_type: #content_type,
                        schema: #schema_ref,
                    }
                });
                if let Some(actual_type) = &variant.actual_type {
                    schemas.push(actual_type);
                } else {
                    schemas.push(item_ty);
                };
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
                    ::std::vec![#(#content),*]
                }

                fn register(registry: &mut #crate_name::registry::Registry) {
                    #(<#schemas as #crate_name::payload::Payload>::register(registry);)*
                }
            }

            impl #impl_generics #crate_name::__private::poem::IntoResponse for #ident #ty_generics #where_clause {
                fn into_response(self) -> #crate_name::__private::poem::Response {
                    match self {
                        #(#into_responses)*
                    }
                }
            }
        }
    };

    Ok(expanded)
}
