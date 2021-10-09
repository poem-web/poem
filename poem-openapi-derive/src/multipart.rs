use darling::{
    ast::Data,
    util::{Ignored, SpannedValue},
    FromDeriveInput, FromField,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ext::IdentExt, Attribute, DeriveInput, Error, Generics, Type};

use crate::{
    common_args::{
        DefaultValue, MaximumValidator, MinimumValidator, RenameRule, RenameRuleExt, RenameTarget,
    },
    error::GeneratorResult,
    utils::{get_crate_name, get_summary_and_description, optional_literal},
    validators::HasValidators,
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
}

impl_has_validators!(MultipartField);

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
            args.rename_all
                .rename(field_ident.unraw().to_string(), RenameTarget::Field)
        });
        let (field_title, field_description) = get_summary_and_description(&field.attrs)?;
        let field_title = optional_literal(&field_title);
        let field_description = optional_literal(&field_description);
        let validators_checker = field
            .validators()
            .create_multipart_field_checker(&crate_name, &field_name)?;
        let validators_update_meta = field.validators().create_update_meta(&crate_name)?;

        fields.push(field_ident);

        deserialize_fields.push(quote! {
            if field.name() == ::std::option::Option::Some(#field_name) {
                #field_ident = match #field_ident {
                    ::std::option::Option::Some(value) => {
                        ::std::option::Option::Some(<#field_ty as #crate_name::types::ParseFromMultipartField>::parse_from_repeated_field(value, field).await.map_err(|err|
                            #crate_name::ParseRequestError::ParseRequestBody { reason: ::std::format!("failed to parse field `{}`: {}", #field_name, err.into_message()) }
                        )?)
                    }
                    ::std::option::Option::None => {
                        ::std::option::Option::Some(<#field_ty as #crate_name::types::ParseFromMultipartField>::parse_from_multipart(::std::option::Option::Some(field)).await.map_err(|err|
                            #crate_name::ParseRequestError::ParseRequestBody { reason: ::std::format!("failed to parse field `{}`: {}", #field_name, err.into_message()) }
                        )?)
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
                                #crate_name::ParseRequestError::ParseRequestBody { reason: ::std::format!("field `{}` is required", #field_name) }
                            )?
                        }
                    };
                });
            }
        }

        let field_meta_default = match &field.default {
            Some(DefaultValue::Default) => {
                quote!(::std::option::Option::Some(#crate_name::types::ToJSON::to_json(&<#field_ty as ::std::default::Default>::default())))
            }
            Some(DefaultValue::Function(func_name)) => {
                quote!(::std::option::Option::Some(#crate_name::types::ToJSON::to_json(&#func_name())))
            }
            None => quote!(::std::option::Option::None),
        };

        meta_fields.push(quote! {{
            let mut schema_ref = <#field_ty as #crate_name::types::Type>::schema_ref();

            if let #crate_name::registry::MetaSchemaRef::Inline(schema) = &mut schema_ref {
                schema.default = #field_meta_default;

                if let ::std::option::Option::Some(title) = #field_title {
                    schema.title = ::std::option::Option::Some(title);
                }

                if let ::std::option::Option::Some(field_description) = #field_description {
                    schema.description = ::std::option::Option::Some(field_description);
                }

                #validators_update_meta
            }

            (#field_name, schema_ref)
        }});

        register_fields.push(quote! {
            <#field_ty as #crate_name::types::Type>::register(registry);
        });

        required_fields.push(quote! {
            if <#field_ty as #crate_name::types::Type>::IS_REQUIRED {
                fields.push(#field_name);
            }
        });
    }

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
                #crate_name::registry::MetaSchemaRef::Inline(schema)
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                #(#register_fields)*
            }
        }

        #[#crate_name::poem::async_trait]
        impl #impl_generics #crate_name::payload::ParsePayload for #ident #ty_generics #where_clause {
            async fn from_request(request: &#crate_name::poem::Request, body: &mut #crate_name::poem::RequestBody) -> ::std::result::Result<Self, #crate_name::ParseRequestError> {
                if body.is_some() {
                    let mut multipart = <#crate_name::poem::web::Multipart as #crate_name::poem::FromRequest>::from_request(request, body).await.map_err(|err| #crate_name::ParseRequestError::ParseRequestBody {
                        reason: ::std::string::ToString::to_string(&err),
                    })?;
                    #(#skip_fields)*
                    #(let mut #fields = ::std::option::Option::None;)*
                    while let ::std::option::Option::Some(field) = multipart.next_field().await.map_err(|err| #crate_name::ParseRequestError::ParseRequestBody { reason: ::std::string::ToString::to_string(&err) })? {
                        #(#deserialize_fields)*
                    }
                    #(#deserialize_none)*
                    ::std::result::Result::Ok(Self { #(#fields,)* #(#skip_idents),* })
                } else {
                    ::std::result::Result::Err(#crate_name::ParseRequestError::ParseRequestBody {
                        reason: ::std::convert::Into::into("expect request body"),
                    })
                }
            }
        }
    };

    Ok(expanded)
}
