use darling::{
    ast::{Data, Fields},
    util::Ignored,
    FromDeriveInput, FromVariant,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ext::IdentExt, Attribute, DeriveInput, Error, Generics, Type};

use crate::{
    common_args::{apply_rename_rule_variant, ExternalDocument, RenameRule},
    error::GeneratorResult,
    utils::{create_object_name, get_crate_name, get_description, optional_literal},
};

#[derive(FromVariant)]
#[darling(attributes(oai), forward_attrs(doc))]
struct UnionItem {
    ident: Ident,
    fields: Fields<Type>,

    #[darling(default)]
    mapping: Option<String>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct UnionArgs {
    ident: Ident,
    attrs: Vec<Attribute>,
    generics: Generics,
    data: Data<UnionItem, Ignored>,

    #[darling(default)]
    internal: bool,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    one_of: bool,
    #[darling(default)]
    discriminator_name: Option<String>,
    #[darling(default)]
    external_docs: Option<ExternalDocument>,
    #[darling(default)]
    rename_all: Option<RenameRule>,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: UnionArgs = UnionArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let ident = &args.ident;
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();
    let oai_typename = args.rename.clone().unwrap_or_else(|| ident.to_string());
    let description = get_description(&args.attrs)?;
    let description = optional_literal(&description);
    let discriminator_name = &args.discriminator_name;

    let e = match &args.data {
        Data::Enum(e) => e,
        _ => return Err(Error::new_spanned(ident, "AnyOf can only be applied to an enum.").into()),
    };

    let mut types = Vec::new();
    let mut from_json = Vec::new();
    let mut to_json = Vec::new();
    let mut mapping = Vec::new();
    let mut create_schemas = Vec::new();
    let mut schemas = Vec::new();

    let required = match &args.discriminator_name {
        Some(discriminator_name) => quote!(::std::vec![#discriminator_name]),
        None => quote!(::std::vec![]),
    };
    let object_name = create_object_name(&crate_name, &oai_typename, &args.generics);

    for variant in e {
        let item_ident = &variant.ident;

        match variant.fields.len() {
            1 => {
                let object_ty = &variant.fields.fields[0];
                let schema_name = quote! {
                    ::std::format!("{}_{}", <Self as #crate_name::types::Type>::name(), <#object_ty as #crate_name::types::Type>::name())
                };
                let mapping_name = match &variant.mapping {
                    Some(mapping) => quote!(::std::string::ToString::to_string(#mapping)),
                    None => {
                        let name = apply_rename_rule_variant(
                            args.rename_all,
                            item_ident.unraw().to_string(),
                        );
                        quote!(::std::string::ToString::to_string(#name))
                    }
                };
                types.push(object_ty);

                if discriminator_name.is_some() {
                    from_json.push(quote! {
                        if ::std::matches!(discriminator_name, ::std::option::Option::Some(discriminator_name) if discriminator_name == &#mapping_name) {
                            return <#object_ty as #crate_name::types::ParseFromJSON>::parse_from_json(::std::option::Option::Some(value))
                                .map(Self::#item_ident)
                                .map_err(#crate_name::types::ParseError::propagate);
                        }
                    });
                } else if !args.one_of {
                    // any of
                    from_json.push(quote! {
                        if let ::std::option::Option::Some(obj) = <#object_ty as #crate_name::types::ParseFromJSON>::parse_from_json(::std::option::Option::Some(::std::clone::Clone::clone(&value)))
                            .map(Self::#item_ident)
                            .ok() {
                            return ::std::result::Result::Ok(obj);
                        }
                    });
                } else {
                    // one of
                    from_json.push(quote! {
                        if let ::std::option::Option::Some(obj) = <#object_ty as #crate_name::types::ParseFromJSON>::parse_from_json(::std::option::Option::Some(::std::clone::Clone::clone(&value)))
                            .map(Self::#item_ident)
                            .ok() {
                            if res_obj.is_some() {
                                return ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value));
                            }
                            res_obj = Some(obj);
                        }
                    });
                }

                if let Some(discriminator_name) = &discriminator_name {
                    to_json.push(quote! {
                        Self::#item_ident(obj) => {
                            let mut value = <#object_ty as #crate_name::types::ToJSON>::to_json(obj);
                            if let ::std::option::Option::Some(obj) = value.as_mut().and_then(|value| value.as_object_mut()) {
                                obj.insert(::std::convert::Into::into(#discriminator_name), ::std::convert::Into::into(#mapping_name));
                            }
                            value
                        }
                    });
                } else {
                    to_json.push(quote! {
                        Self::#item_ident(obj) => <#object_ty as #crate_name::types::ToJSON>::to_json(obj)
                    });
                }

                mapping.push(quote! {
                    (#mapping_name, ::std::format!("#/components/schemas/{}", #schema_name))
                });

                if let Some(discriminator_name) = &args.discriminator_name {
                    create_schemas.push(quote! {
                        let schema = #crate_name::registry::MetaSchema {
                            all_of: ::std::vec![
                                #crate_name::registry::MetaSchemaRef::Inline(::std::boxed::Box::new(#crate_name::registry::MetaSchema {
                                    required: #required,
                                    properties: ::std::vec![
                                        (
                                            #discriminator_name,
                                            #crate_name::registry::MetaSchemaRef::merge(
                                                <::std::string::String as #crate_name::types::Type>::schema_ref(),
                                                #crate_name::registry::MetaSchema {
                                                    example: ::std::option::Option::Some(::std::convert::Into::into(#mapping_name)),
                                                    ..#crate_name::registry::MetaSchema::ANY
                                                }
                                            )
                                        )
                                    ],
                                    ..#crate_name::registry::MetaSchema::new("object")
                                })),
                                <#object_ty as #crate_name::types::Type>::schema_ref(),
                            ],
                            ..#crate_name::registry::MetaSchema::ANY
                        };
                        registry.schemas.insert(#schema_name, schema);
                    });

                    schemas.push(quote! {
                        #crate_name::registry::MetaSchemaRef::Reference(#schema_name)
                    });
                } else {
                    schemas.push(quote! {
                        <#object_ty as #crate_name::types::Type>::schema_ref()
                    });
                }
            }
            _ => {
                return Err(
                    Error::new_spanned(&variant.ident, "Incorrect oneof definition.").into(),
                )
            }
        }
    }

    let discriminator = match &args.discriminator_name {
        Some(discriminator_name) => quote! {
            ::std::option::Option::Some(#crate_name::registry::MetaDiscriminatorObject {
                property_name: #discriminator_name,
                mapping: ::std::vec![#(#mapping),*],
            })
        },
        None => quote!(::std::option::Option::None),
    };

    let parse_from_json = match &args.discriminator_name {
        Some(discriminator_name) => quote! {
            let discriminator_name = value.as_object().and_then(|obj| obj.get(#discriminator_name));
            #(#from_json)*
            ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value))
        },
        // anyof
        None if !args.one_of => quote! {
            #(#from_json)*
            ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value))
        },
        // oneof
        None => quote! {
            let mut res_obj = ::std::option::Option::None;
            #(#from_json)*
            match res_obj {
                ::std::option::Option::Some(res_obj) => ::std::result::Result::Ok(res_obj),
                ::std::option::Option::None => ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value)),
            }
        },
    };

    let external_docs = match &args.external_docs {
        Some(external_docs) => {
            let s = external_docs.to_token_stream(&crate_name);
            quote!(::std::option::Option::Some(#s))
        }
        None => quote!(::std::option::Option::None),
    };

    let one_of = if args.one_of {
        quote!(::std::vec![#(#schemas),*])
    } else {
        quote!(::std::vec![])
    };

    let any_of = if !args.one_of {
        quote!(::std::vec![#(#schemas),*])
    } else {
        quote!(::std::vec![])
    };

    let meta = quote! {
        #crate_name::registry::MetaSchema {
            ty: "object",
            description: #description,
            external_docs: #external_docs,
            one_of: #one_of,
            any_of: #any_of,
            discriminator: #discriminator,
            ..#crate_name::registry::MetaSchema::ANY
        }
    };

    let expanded = quote! {
        impl #impl_generics #crate_name::types::Type for #ident #ty_generics #where_clause {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> ::std::borrow::Cow<'static, str> {
                ::std::convert::Into::into(#object_name)
            }

            fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                #crate_name::registry::MetaSchemaRef::Reference(<Self as #crate_name::types::Type>::name().into_owned())
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                registry.create_schema::<Self, _>(<Self as #crate_name::types::Type>::name().into_owned(), |registry| {
                    #(<#types as #crate_name::types::Type>::register(registry);)*
                    #(#create_schemas)*
                    #meta
                });
            }

            fn as_raw_value(&self) -> ::std::option::Option<&Self::RawValueType> {
                ::std::option::Option::Some(self)
            }

            fn raw_element_iter<'a>(&'a self) -> ::std::boxed::Box<dyn ::std::iter::Iterator<Item = &'a Self::RawElementValueType> + 'a> {
                ::std::boxed::Box::new(::std::iter::IntoIterator::into_iter(self.as_raw_value()))
            }
        }

        impl #impl_generics #crate_name::types::ParseFromJSON for #ident #ty_generics #where_clause {
            fn parse_from_json(value: ::std::option::Option<#crate_name::__private::serde_json::Value>) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>> {
                let value = value.unwrap_or_default();
                #parse_from_json
            }
        }

        impl #impl_generics #crate_name::types::ToJSON for #ident #ty_generics #where_clause {
            fn to_json(&self) -> ::std::option::Option<#crate_name::__private::serde_json::Value> {
                match self {
                    #(#to_json),*
                }
            }
        }
    };

    Ok(expanded)
}
