use darling::{
    ast::{Data, Fields},
    util::Ignored,
    FromDeriveInput, FromField, FromVariant,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{Attribute, DeriveInput, Error, Generics, Path, Type};

use crate::{
    error::GeneratorResult,
    utils::{get_crate_name, get_description, optional_literal},
};

#[derive(FromField)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ResponseField {
    ty: Type,
    attrs: Vec<Attribute>,

    #[darling(default)]
    header: Option<String>,
}

#[derive(FromVariant)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ResponseItem {
    ident: Ident,
    attrs: Vec<Attribute>,
    fields: Fields<ResponseField>,

    #[darling(default)]
    status: Option<u16>,
    #[darling(default)]
    content_type: Option<String>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ResponseArgs {
    ident: Ident,
    data: Data<ResponseItem, Ignored>,
    generics: Generics,

    #[darling(default)]
    internal: bool,
    #[darling(default)]
    bad_request_handler: Option<Path>,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: ResponseArgs = ResponseArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();
    let ident = &args.ident;
    let e = match &args.data {
        Data::Enum(e) => e,
        _ => {
            return Err(
                Error::new_spanned(ident, "Response can only be applied to an enum.").into(),
            )
        }
    };

    let mut into_responses = Vec::new();
    let mut responses_meta = Vec::new();
    let mut schemas = Vec::new();

    for variant in e {
        let item_ident = &variant.ident;
        let item_description = get_description(&variant.attrs)?;
        let item_description = optional_literal(&item_description);
        let (values, headers) = parse_fields(&variant.fields);

        let mut match_headers = Vec::new();
        let mut with_headers = Vec::new();
        let mut meta_headers = Vec::new();

        for (idx, header) in headers.iter().enumerate() {
            let ident = quote::format_ident!("__p{}", idx);
            let header_name = header.header.as_ref().unwrap().to_uppercase();
            let header_ty = &header.ty;
            let header_desc = optional_literal(&get_description(&header.attrs)?);

            with_headers.push(quote! {{
                if let Some(header) = #crate_name::types::ToHeader::to_header(&#ident) {
                    resp.headers_mut().insert(#header_name, header);
                }
            }});
            match_headers.push(ident);
            meta_headers.push(quote! {
                #crate_name::registry::MetaHeader {
                    name: #header_name,
                    description: #header_desc,
                    required: <#header_ty as #crate_name::types::Type>::IS_REQUIRED,
                    schema: <#header_ty as #crate_name::types::Type>::schema_ref(),
                }
            });
        }

        fn update_content(
            crate_name: &TokenStream,
            content_type: Option<&str>,
            payload_ty: &Type,
        ) -> (TokenStream, TokenStream) {
            let content_type_value = match content_type {
                Some(content_type) => quote!(#content_type),
                None => quote!(<#payload_ty as #crate_name::payload::Payload>::CONTENT_TYPE),
            };
            let update_content_type = match content_type {
                Some(content_type) => quote! {
                    resp.headers_mut().insert(#crate_name::__private::poem::http::header::CONTENT_TYPE,
                        #crate_name::__private::poem::http::HeaderValue::from_static(#content_type));
                },
                None => quote!(),
            };
            (content_type_value, update_content_type)
        }

        match values.len() {
            2 => {
                // #[oai(default)]
                // Item(StatusCode, payload)
                let payload_ty = &values[1].ty;
                let (content_type, update_content_type) =
                    update_content(&crate_name, variant.content_type.as_deref(), payload_ty);
                into_responses.push(quote! {
                    #ident::#item_ident(status, payload, #(#match_headers),*) => {
                        let mut resp = #crate_name::__private::poem::IntoResponse::into_response(payload);
                        resp.set_status(status);
                        #(#with_headers)*
                        #update_content_type
                        resp
                    }
                });
                responses_meta.push(quote! {
                    #crate_name::registry::MetaResponse {
                        description: #item_description.unwrap_or_default(),
                        status: ::std::option::Option::None,
                        content: ::std::vec![#crate_name::registry::MetaMediaType {
                            content_type: #content_type,
                            schema: <#payload_ty as #crate_name::payload::Payload>::schema_ref(),
                        }],
                        headers: ::std::vec![#(#meta_headers),*],
                    }
                });
                schemas.push(payload_ty);
            }
            1 => {
                // #[oai(status = 200)]
                // Item(payload)
                let payload_ty = &values[0].ty;
                let (content_type, update_content_type) =
                    update_content(&crate_name, variant.content_type.as_deref(), payload_ty);
                let status = get_status(variant.ident.span(), variant.status)?;
                into_responses.push(quote! {
                    #ident::#item_ident(payload, #(#match_headers),*) => {
                        let mut resp = #crate_name::__private::poem::IntoResponse::into_response(payload);
                        resp.set_status(#crate_name::__private::poem::http::StatusCode::from_u16(#status).unwrap());
                        #(#with_headers)*
                        #update_content_type
                        resp
                    }
                });
                responses_meta.push(quote! {
                    #crate_name::registry::MetaResponse {
                        description: #item_description.unwrap_or_default(),
                        status: ::std::option::Option::Some(#status),
                        content: ::std::vec![#crate_name::registry::MetaMediaType {
                            content_type: #content_type,
                            schema: <#payload_ty as #crate_name::payload::Payload>::schema_ref(),
                        }],
                        headers: ::std::vec![#(#meta_headers),*],
                    }
                });
                schemas.push(payload_ty);
            }
            0 => {
                // #[oai(status = 200)]
                // Item
                let status = get_status(variant.ident.span(), variant.status)?;
                let item = if !headers.is_empty() {
                    quote!(#ident::#item_ident(#(#match_headers),*))
                } else {
                    quote!(#ident::#item_ident)
                };
                into_responses.push(quote! {
                    #item => {
                        let status = #crate_name::__private::poem::http::StatusCode::from_u16(#status).unwrap();
                        #[allow(unused_mut)]
                        let mut resp = #crate_name::__private::poem::IntoResponse::into_response(status);
                        #(#with_headers)*
                        resp
                    }
                });
                responses_meta.push(quote! {
                    #crate_name::registry::MetaResponse {
                        description: #item_description.unwrap_or_default(),
                        status: ::std::option::Option::Some(#status),
                        content: ::std::vec![],
                        headers: ::std::vec![#(#meta_headers),*],
                    }
                });
            }
            _ => {
                return Err(
                    Error::new_spanned(&variant.ident, "Incorrect response definition.").into(),
                )
            }
        }
    }

    let bad_request_handler_const = match &args.bad_request_handler {
        Some(_) => quote!(
            const BAD_REQUEST_HANDLER: bool = true;
        ),
        None => quote!(
            const BAD_REQUEST_HANDLER: bool = false;
        ),
    };
    let bad_request_handler = args.bad_request_handler.as_ref().map(|path| {
        quote! {
            fn from_parse_request_error(err: #crate_name::__private::poem::Error) -> Self {
                #path(err)
            }
        }
    });

    let expanded = {
        quote! {
            impl #impl_generics #crate_name::__private::poem::IntoResponse for #ident #ty_generics #where_clause {
                fn into_response(self) -> #crate_name::__private::poem::Response {
                    match self {
                        #(#into_responses)*
                    }
                }
            }

            impl #impl_generics #crate_name::ApiResponse for #ident #ty_generics #where_clause {
                #bad_request_handler_const

                fn meta() -> #crate_name::registry::MetaResponses {
                    #crate_name::registry::MetaResponses {
                        responses: ::std::vec![#(#responses_meta),*],
                    }
                }

                fn register(registry: &mut #crate_name::registry::Registry) {
                    #(<#schemas as #crate_name::payload::Payload>::register(registry);)*
                }

                #bad_request_handler
            }
        }
    };

    Ok(expanded)
}

fn get_status(span: Span, status: Option<u16>) -> GeneratorResult<TokenStream> {
    let status =
        status.ok_or_else(|| Error::new(span, "Response can only be applied to an enum."))?;
    if !(100..1000).contains(&status) {
        return Err(Error::new(
            span,
            "Invalid status code, it must be greater or equal to 100 and less than 1000.",
        )
        .into());
    }
    Ok(quote!(#status))
}

fn parse_fields(fields: &Fields<ResponseField>) -> (Vec<&ResponseField>, Vec<&ResponseField>) {
    let mut values = Vec::new();
    let mut headers = Vec::new();

    for field in &fields.fields {
        if field.header.is_some() {
            headers.push(field);
        } else {
            values.push(field);
        }
    }

    (values, headers)
}
