use std::str::FromStr;

use darling::{
    ast::{Data, Fields},
    util::{Ignored, SpannedValue},
    FromDeriveInput, FromVariant,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Attribute, DeriveInput, Error, Generics, Type};

use crate::{
    error::GeneratorResult,
    utils::{get_crate_name, get_description, optional_literal},
};

#[derive(FromVariant)]
#[darling(attributes(oai), forward_attrs(doc))]
struct RequestItem {
    ident: Ident,
    fields: Fields<Type>,

    #[darling(default)]
    content_type: Option<SpannedValue<String>>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct RequestArgs {
    ident: Ident,
    attrs: Vec<Attribute>,
    generics: Generics,
    data: Data<RequestItem, Ignored>,

    #[darling(default)]
    internal: bool,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: RequestArgs = RequestArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();
    let ident = &args.ident;
    let e = match &args.data {
        Data::Enum(e) => e,
        _ => {
            return Err(Error::new_spanned(ident, "Request can only be applied to an enum.").into())
        }
    };
    let description = get_description(&args.attrs)?;
    let description = optional_literal(&description);

    let mut from_requests = Vec::new();
    let mut content = Vec::new();
    let mut schemas = Vec::new();

    let impl_generics = {
        let mut s = quote!(#impl_generics).to_string();
        match s.find('<') {
            Some(pos) => {
                s.insert_str(pos + 1, "'__request,");
                TokenStream::from_str(&s).unwrap()
            }
            _ => quote!(<'__request>),
        }
    };

    for variant in e {
        let item_ident = &variant.ident;

        match variant.fields.len() {
            1 => {
                // Item(payload)
                let payload_ty = &variant.fields.fields[0];
                let content_type = match &variant.content_type {
                    Some(content_type) => {
                        let content_type = &**content_type;
                        quote!(#content_type)
                    }
                    None => {
                        quote!(<#payload_ty as #crate_name::payload::Payload>::CONTENT_TYPE)
                    }
                };
                let check_content_type = match &variant.content_type {
                    Some(content_type) => {
                        let content_type = &**content_type;
                        quote!(content_type == #content_type)
                    }
                    None => {
                        quote!(<#payload_ty as #crate_name::payload::Payload>::check_content_type(content_type))
                    }
                };
                from_requests.push(quote! {
                    if #check_content_type {
                        return ::std::result::Result::Ok(#ident::#item_ident(
                            <#payload_ty as #crate_name::payload::ParsePayload>::from_request(request, body).await?
                        ));
                    }
                });
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
            impl #impl_generics #crate_name::ApiExtractor<'__request> for #ident #ty_generics #where_clause {
                const TYPE: #crate_name::ApiExtractorType = #crate_name::ApiExtractorType::RequestObject;

                type ParamType = ();
                type ParamRawType = ();

                fn register(registry: &mut #crate_name::registry::Registry) {
                    #(<#schemas as #crate_name::payload::Payload>::register(registry);)*
                }

                fn request_meta() -> ::std::option::Option<#crate_name::registry::MetaRequest> {
                    ::std::option::Option::Some(#crate_name::registry::MetaRequest {
                        description: #description,
                        content: ::std::vec![#(#content),*],
                        required: true,
                    })
                }

                async fn from_request(
                    request: &'__request #crate_name::__private::poem::Request,
                    body: &mut #crate_name::__private::poem::RequestBody,
                    _param_opts: #crate_name::ExtractParamOptions<Self::ParamType>,
                ) -> #crate_name::__private::poem::Result<Self> {
                    use ::std::str::FromStr;

                    match request.content_type() {
                        ::std::option::Option::Some(content_type) => {
                            #(#from_requests)*
                            ::std::result::Result::Err(
                                ::std::convert::Into::into(#crate_name::error::ContentTypeError::NotSupported {
                                    content_type: ::std::string::ToString::to_string(content_type),
                            }))
                        }
                        ::std::option::Option::None => {
                            ::std::result::Result::Err(::std::convert::Into::into(#crate_name::error::ContentTypeError::ExpectContentType))
                        }
                    }
                }
            }
        }
    };

    Ok(expanded)
}
