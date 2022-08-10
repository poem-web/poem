use std::str::FromStr;

use darling::{ast::Data, util::Ignored, FromDeriveInput, FromField};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ext::IdentExt, Attribute, DeriveInput, Error, Generics, Type};

use crate::{
    common_args::{apply_rename_rule_field, DefaultValue, RenameRule},
    error::GeneratorResult,
    utils::{get_crate_name, get_description, optional_literal},
    validators::Validators,
};

#[derive(FromField)]
#[darling(attributes(oai), forward_attrs(doc))]
struct MultipartField {
    ident: Option<Ident>,
    ty: Type,
    attrs: Vec<Attribute>,

    #[darling(default)]
    skip: bool,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    default: Option<DefaultValue>,
    #[darling(default)]
    validator: Option<Validators>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai))]
struct MultipartArgs {
    ident: Ident,
    generics: Generics,
    data: Data<Ignored, MultipartField>,

    #[darling(default)]
    internal: bool,
    #[darling(default)]
    rename_all: Option<RenameRule>,
    #[darling(default)]
    deny_unknown_fields: bool,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: MultipartArgs = MultipartArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();
    let ident = &args.ident;

    let s = match &args.data {
        Data::Struct(s) => s,
        _ => {
            return Err(
                Error::new_spanned(ident, "Multipart can only be applied to an struct.").into(),
            )
        }
    };

    let mut skip_fields = Vec::new();
    let mut skip_idents = Vec::new();
    let mut deserialize_fields = Vec::new();
    let mut deserialize_none = Vec::new();
    let mut fields = Vec::new();
    let mut meta_fields = Vec::new();
    let mut register_fields = Vec::new();
    let mut required_fields = Vec::new();

    for field in &s.fields {
        let field_ident = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;

        if field.skip {
            skip_fields.push(quote! {
                let #field_ident: #field_ty = ::std::default::Default::default();
            });
            skip_idents.push(field_ident);
            continue;
        }

        let field_name = field.rename.clone().unwrap_or_else(|| {
            apply_rename_rule_field(args.rename_all, field_ident.unraw().to_string())
        });
        let field_description = get_description(&field.attrs)?;
        let field_description = optional_literal(&field_description);
        let validators = field.validator.clone().unwrap_or_default();
        let validators_checker =
            validators.create_multipart_field_checker(&crate_name, &field_name)?;
        let validators_update_meta = validators.create_update_meta(&crate_name)?;

        fields.push(field_ident);

        let parse_err = quote! {{
            #crate_name::error::ParseMultipartError {
                reason: ::std::format!("failed to parse field `{}`: {}", #field_name, err.into_message()),
            }
        }};

        deserialize_fields.push(quote! {
            if field.name() == ::std::option::Option::Some(#field_name) {
                #field_ident = match #field_ident {
                    ::std::option::Option::Some(value) => {
                        ::std::option::Option::Some(<#field_ty as #crate_name::types::ParseFromMultipartField>::parse_from_repeated_field(value, field).await.map_err(|err| #parse_err )?)
                    }
                    ::std::option::Option::None => {
                        ::std::option::Option::Some(<#field_ty as #crate_name::types::ParseFromMultipartField>::parse_from_multipart(::std::option::Option::Some(field)).await.map_err(|err| #parse_err )?)
                    }
                };
                continue;
            }
        });

        match &field.default {
            Some(default_value) => {
                let default_value = match default_value {
                    DefaultValue::Default => {
                        quote!(<#field_ty as ::std::default::Default>::default())
                    }
                    DefaultValue::Function(func_name) => quote!(#func_name()),
                };

                deserialize_none.push(quote! {
                    let #field_ident = match #field_ident {
                        ::std::option::Option::Some(value) => {
                            #validators_checker
                            value
                        },
                        ::std::option::Option::None => #default_value,
                    };
                });
            }
            None => {
                deserialize_none.push(quote! {
                    let #field_ident = match #field_ident {
                        ::std::option::Option::Some(value) => {
                            #validators_checker
                            value
                        },
                        ::std::option::Option::None => {
                            <#field_ty as #crate_name::types::ParseFromMultipartField>::parse_from_multipart(::std::option::Option::None).await.map_err(|_|
                                #crate_name::error::ParseMultipartError {
                                    reason: ::std::format!("field `{}` is required", #field_name),
                                }
                            )?
                        }
                    };
                });
            }
        }

        let has_default = field.default.is_some();
        let field_meta_default = match &field.default {
            Some(DefaultValue::Default) => {
                quote!(#crate_name::types::ToJSON::to_json(&<#field_ty as ::std::default::Default>::default()))
            }
            Some(DefaultValue::Function(func_name)) => {
                quote!(#crate_name::types::ToJSON::to_json(&#func_name()))
            }
            None => quote!(::std::option::Option::None),
        };

        meta_fields.push(quote! {{
            let original_schema = <#field_ty as #crate_name::types::Type>::schema_ref();
            let mut patch_schema = {
                let mut schema = #crate_name::registry::MetaSchema::ANY;
                schema.default = #field_meta_default;

                if let ::std::option::Option::Some(field_description) = #field_description {
                    schema.description = ::std::option::Option::Some(field_description);
                }

                #validators_update_meta
                schema
            };

            (#field_name, original_schema.merge(patch_schema))
        }});

        register_fields.push(quote! {
            <#field_ty as #crate_name::types::Type>::register(registry);
        });

        required_fields.push(quote! {
            if <#field_ty as #crate_name::types::Type>::IS_REQUIRED && !#has_default {
                fields.push(#field_name);
            }
        });
    }

    let extractor_impl_generics = {
        let mut s = quote!(#impl_generics).to_string();
        match s.find('<') {
            Some(pos) => {
                s.insert_str(pos + 1, "'__request,");
                TokenStream::from_str(&s).unwrap()
            }
            _ => quote!(<'__request>),
        }
    };

    let deny_unknown_fields = if args.deny_unknown_fields {
        Some(quote! {
            if let ::std::option::Option::Some(name) = field.name() {
                return ::std::result::Result::Err(::std::convert::Into::into(#crate_name::error::ParseMultipartError {
                    reason: ::std::format!("unknown field `{}`", name),
                }));
            }
        })
    } else {
        None
    };

    let expanded = quote! {
        impl #impl_generics #crate_name::payload::Payload for #ident #ty_generics #where_clause {
            const CONTENT_TYPE: &'static str = "multipart/form-data";

            fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                let schema = #crate_name::registry::MetaSchema {
                    required: {
                        #[allow(unused_mut)]
                        let mut fields = ::std::vec::Vec::new();
                        #(#required_fields)*
                        fields
                    },
                    properties: ::std::vec![#(#meta_fields),*],
                    ..#crate_name::registry::MetaSchema::new("object")
                };
                #crate_name::registry::MetaSchemaRef::Inline(Box::new(schema))
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                #(#register_fields)*
            }
        }

        #[#crate_name::__private::poem::async_trait]
        impl #impl_generics #crate_name::payload::ParsePayload for #ident #ty_generics #where_clause {
            const IS_REQUIRED: bool = true;

            async fn from_request(request: &#crate_name::__private::poem::Request, body: &mut #crate_name::__private::poem::RequestBody) -> #crate_name::__private::poem::Result<Self> {
                let mut multipart = <#crate_name::__private::poem::web::Multipart as #crate_name::__private::poem::FromRequest>::from_request(request, body).await?;
                #(#skip_fields)*
                #(let mut #fields = ::std::option::Option::None;)*
                while let ::std::option::Option::Some(field) = multipart.next_field().await? {
                    #(#deserialize_fields)*
                    #deny_unknown_fields
                }
                #(#deserialize_none)*
                ::std::result::Result::Ok(Self { #(#fields,)* #(#skip_idents),* })
            }
        }

        #[#crate_name::__private::poem::async_trait]
        impl #extractor_impl_generics #crate_name::ApiExtractor<'__request> for #ident #ty_generics #where_clause {
            const TYPE: #crate_name::ApiExtractorType = #crate_name::ApiExtractorType::RequestObject;

            type ParamType = ();
            type ParamRawType = ();

            fn register(registry: &mut #crate_name::registry::Registry) {
                <Self as #crate_name::payload::Payload>::register(registry);
            }

            fn request_meta() -> ::std::option::Option<#crate_name::registry::MetaRequest> {
                ::std::option::Option::Some(#crate_name::registry::MetaRequest {
                    description: ::std::option::Option::None,
                    content: ::std::vec![#crate_name::registry::MetaMediaType {
                        content_type: <Self as #crate_name::payload::Payload>::CONTENT_TYPE,
                        schema: <Self as #crate_name::payload::Payload>::schema_ref(),
                    }],
                    required: <Self as #crate_name::payload::ParsePayload>::IS_REQUIRED,
                })
            }

            async fn from_request(
                request: &'__request #crate_name::__private::poem::Request,
                body: &mut #crate_name::__private::poem::RequestBody,
                _param_opts: #crate_name::ExtractParamOptions<Self::ParamType>,
            ) -> #crate_name::__private::poem::Result<Self> {
                match request.content_type() {
                    ::std::option::Option::Some(content_type) => {
                        let mime: #crate_name::__private::mime::Mime = match content_type.parse() {
                            ::std::result::Result::Ok(mime) => mime,
                            ::std::result::Result::Err(_) => {
                                return ::std::result::Result::Err(::std::convert::Into::into(#crate_name::error::ContentTypeError::NotSupported {
                                    content_type: ::std::string::ToString::to_string(&content_type),
                                }));
                            }
                        };

                        if mime.essence_str() != <Self as #crate_name::payload::Payload>::CONTENT_TYPE {
                            return ::std::result::Result::Err(::std::convert::Into::into(#crate_name::error::ContentTypeError::NotSupported {
                                content_type: ::std::string::ToString::to_string(&content_type),
                            }));
                        }

                        <Self as #crate_name::payload::ParsePayload>::from_request(request, body).await
                    }
                    ::std::option::Option::None => ::std::result::Result::Err(::std::convert::Into::into(#crate_name::error::ContentTypeError::ExpectContentType)),
                }
            }
        }
    };

    Ok(expanded)
}
