use darling::{
    ast::{Data, Fields},
    util::Ignored,
    FromDeriveInput, FromField, FromVariant,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{Attribute, DeriveInput, Error, Generics, Path, Type};

use crate::{
    common_args::ExtraHeader,
    error::GeneratorResult,
    utils::{get_crate_name, get_description, optional_literal, optional_literal_string},
};

#[derive(FromField)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ResponseField {
    ty: Type,
    attrs: Vec<Attribute>,

    #[darling(default)]
    header: Option<String>,
    #[darling(default)]
    deprecated: bool,
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
    #[darling(default, multiple, rename = "header")]
    headers: Vec<ExtraHeader>,
    #[darling(default)]
    actual_type: Option<Type>,
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
    #[darling(default, multiple, rename = "header")]
    headers: Vec<ExtraHeader>,
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
        let (values, headers) = parse_fields(&variant.fields)?;

        let mut match_headers = Vec::new();
        let mut with_headers = Vec::new();
        let mut meta_headers = Vec::new();

        // headers
        for (idx, header) in headers.iter().enumerate() {
            let ident = quote::format_ident!("__p{}", idx);
            let header_name = header.header.as_ref().unwrap().to_uppercase();
            let header_ty = &header.ty;
            let header_desc = optional_literal_string(&get_description(&header.attrs)?);
            let deprecated = header.deprecated;

            with_headers.push(quote! {{
                if let Some(header) = #crate_name::types::ToHeader::to_header(&#ident) {
                    resp.headers_mut().insert(#header_name, header);
                }
            }});
            match_headers.push(ident);
            meta_headers.push(quote! {
                #crate_name::registry::MetaHeader {
                    name: ::std::string::ToString::to_string(#header_name),
                    description: #header_desc,
                    required: <#header_ty as #crate_name::types::Type>::IS_REQUIRED,
                    deprecated: #deprecated,
                    schema: <#header_ty as #crate_name::types::Type>::schema_ref(),
                }
            });
        }

        // extra headers
        for header in args.headers.iter().chain(&variant.headers) {
            let name = header.name.to_uppercase();
            let description = optional_literal_string(&header.description);
            let ty = match syn::parse_str::<Type>(&header.ty) {
                Ok(ty) => ty,
                Err(_) => return Err(Error::new(header.ty.span(), "Invalid type").into()),
            };
            let deprecated = header.deprecated;

            meta_headers.push(quote! {
                #crate_name::registry::MetaHeader {
                    name: ::std::string::ToString::to_string(#name),
                    description: #description,
                    required: <#ty as #crate_name::types::Type>::IS_REQUIRED,
                    deprecated: #deprecated,
                    schema: <#ty as #crate_name::types::Type>::schema_ref(),
                }
            });
        }

        fn update_content_type(
            crate_name: &TokenStream,
            content_type: Option<&str>,
            actual_type: Option<&Type>,
        ) -> (TokenStream, TokenStream) {
            let (update_response_content_type, update_meta_content_type) = if let Some(
                content_type,
            ) = content_type
            {
                (
                    quote! {
                        resp.headers_mut().insert(#crate_name::__private::poem::http::header::CONTENT_TYPE,
                            #crate_name::__private::poem::http::HeaderValue::from_static(#content_type));
                    },
                    quote! {
                        if let Some(mt) = content.get_mut(0) {
                            mt.content_type = #content_type;
                        }
                    },
                )
            } else if let Some(actual_type) = actual_type {
                (
                    quote! {
                        resp.headers_mut().insert(#crate_name::__private::poem::http::header::CONTENT_TYPE,
                            #crate_name::__private::poem::http::HeaderValue::from_static(<#actual_type as #crate_name::payload::Payload>::CONTENT_TYPE)
                        );
                    },
                    quote! {
                        if let Some(mt) = content.get_mut(0) {
                            mt.content_type = <#actual_type as #crate_name::payload::Payload>::CONTENT_TYPE;
                            mt.schema = <#actual_type as #crate_name::payload::Payload>::schema_ref();
                        }
                    },
                )
            } else {
                (quote! {}, quote! {})
            };

            (update_response_content_type, update_meta_content_type)
        }

        match values.len() {
            2 => {
                // Item(StatusCode, media)
                let media_ty = &values[1].ty;
                let (update_response_content_type, update_meta_content_type) = update_content_type(
                    &crate_name,
                    variant.content_type.as_deref(),
                    variant.actual_type.as_ref(),
                );
                into_responses.push(quote! {
                    #ident::#item_ident(status, media, #(#match_headers),*) => {
                        let mut resp = #crate_name::__private::poem::IntoResponse::into_response(media);
                        resp.set_status(status);
                        #(#with_headers)*
                        #update_response_content_type
                        resp
                    }
                });
                responses_meta.push(quote! {
                    #crate_name::registry::MetaResponse {
                        description: #item_description.unwrap_or_default(),
                        status: ::std::option::Option::None,
                        content: {
                            let mut content = <#media_ty as #crate_name::ResponseContent>::media_types();
                            #update_meta_content_type
                            content
                        },
                        headers: ::std::vec![#(#meta_headers),*],
                    }
                });
                if let Some(actual_type) = variant.actual_type.as_ref() {
                    schemas.push(actual_type);
                } else {
                    schemas.push(media_ty);
                }
            }
            1 => {
                // #[oai(status = 200)]
                // Item(media)
                let media_ty = &values[0].ty;
                let status = get_status(variant.ident.span(), variant.status)?;
                let (update_response_content_type, update_meta_content_type) = update_content_type(
                    &crate_name,
                    variant.content_type.as_deref(),
                    variant.actual_type.as_ref(),
                );
                into_responses.push(quote! {
                    #ident::#item_ident(media, #(#match_headers),*) => {
                        let mut resp = #crate_name::__private::poem::IntoResponse::into_response(media);
                        resp.set_status(#crate_name::__private::poem::http::StatusCode::from_u16(#status).unwrap());
                        #(#with_headers)*
                        #update_response_content_type
                        resp
                    }
                });
                responses_meta.push(quote! {
                    #crate_name::registry::MetaResponse {
                        description: #item_description.unwrap_or_default(),
                        status: ::std::option::Option::Some(#status),
                        content: {
                            let mut content = <#media_ty as #crate_name::ResponseContent>::media_types();
                            #update_meta_content_type
                            content
                        },
                        headers: ::std::vec![#(#meta_headers),*],
                    }
                });
                if let Some(actual_type) = variant.actual_type.as_ref() {
                    schemas.push(actual_type);
                } else {
                    schemas.push(media_ty);
                }
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
                        responses: ::std::vec![#(#responses_meta),*]
                    }
                }

                fn register(registry: &mut #crate_name::registry::Registry) {
                    #(<#schemas as #crate_name::ResponseContent>::register(registry);)*
                }

                #bad_request_handler
            }

            impl #impl_generics ::std::convert::From<#ident #ty_generics> for #crate_name::__private::poem::Error #where_clause {
                fn from(resp: #ident #ty_generics) -> #crate_name::__private::poem::Error {
                    use #crate_name::__private::poem::IntoResponse;
                    #crate_name::__private::poem::Error::from_response(resp.into_response())
                }
            }
        }
    };

    Ok(expanded)
}

fn get_status(span: Span, status: Option<u16>) -> GeneratorResult<TokenStream> {
    let status = status.ok_or_else(|| Error::new(span, "Missing status attribute"))?;
    if !(100..1000).contains(&status) {
        return Err(Error::new(
            span,
            "Invalid status code, it must be greater or equal to 100 and less than 1000.",
        )
        .into());
    }
    Ok(quote!(#status))
}

fn parse_fields(
    fields: &Fields<ResponseField>,
) -> syn::Result<(Vec<&ResponseField>, Vec<&ResponseField>)> {
    let mut values = Vec::new();
    let mut headers = Vec::new();

    for field in &fields.fields {
        if field.header.is_some() {
            headers.push(field);
        } else {
            values.push(field);
        }
    }

    Ok((values, headers))
}
