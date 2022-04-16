use darling::{
    ast::{Data, Fields},
    util::{Ignored, SpannedValue},
    FromDeriveInput, FromVariant,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Attribute, DeriveInput, Error, GenericParam, Generics, Type};

use crate::{
    common_args::{ConcreteType, ExternalDocument},
    error::GeneratorResult,
    utils::{get_crate_name, get_description, optional_literal},
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
    inline: SpannedValue<bool>,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    one_of: bool,
    #[darling(default)]
    discriminator_name: Option<String>,
    #[darling(default)]
    external_docs: Option<ExternalDocument>,
    #[darling(default, multiple, rename = "concrete")]
    concretes: Vec<ConcreteType>,
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

    if *args.inline && !args.concretes.is_empty() {
        return Err(Error::new(
            args.inline.span(),
            "Inline objects cannot have the `concretes` attribute.",
        )
        .into());
    }

    let is_generic_union = args
        .generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Type(_)));
    if is_generic_union && !*args.inline && args.concretes.is_empty() {
        return Err(Error::new(
            args.ident.span(),
            "Generic objects either specify the `inline` attribute, or specify a name for each concrete type using the `concretes` attribute.",
        )
        .into());
    }

    let mut types = Vec::new();
    let mut from_json = Vec::new();
    let mut to_json = Vec::new();
    let mut mapping = Vec::new();
    let mut names = Vec::new();
    let mut schemas = Vec::new();

    let required = match &args.discriminator_name {
        Some(discriminator_name) => quote!(::std::vec![#discriminator_name]),
        None => quote!(::std::vec![]),
    };

    for variant in e {
        let item_ident = &variant.ident;

        match variant.fields.len() {
            1 => {
                let object_ty = &variant.fields.fields[0];
                let mapping_name = match &variant.mapping {
                    Some(mapping) => quote!(#mapping),
                    None => {
                        quote!(::std::convert::AsRef::as_ref(&<#object_ty as #crate_name::types::Type>::name()))
                    }
                };
                names.push(quote!(#mapping_name));

                types.push(object_ty);

                if discriminator_name.is_some() {
                    from_json.push(quote! {
                        if ::std::matches!(discriminator_name, ::std::option::Option::Some(discriminator_name) if discriminator_name == #mapping_name) {
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

                if variant.mapping.is_some() {
                    mapping.push(quote! {
                        (#mapping_name, format!("#/components/schemas/{}", <#object_ty as #crate_name::types::Type>::schema_ref().unwrap_reference()))
                    });
                }

                if let Some(discriminator_name) = &args.discriminator_name {
                    schemas.push(quote! {
                        #crate_name::registry::MetaSchemaRef::Inline(::std::boxed::Box::new(#crate_name::registry::MetaSchema {
                            required: #required,
                            all_of: ::std::vec![
                                <#object_ty as #crate_name::types::Type>::schema_ref(),
                                #crate_name::registry::MetaSchemaRef::Inline(::std::boxed::Box::new(#crate_name::registry::MetaSchema {
                                    title: ::std::option::Option::Some(::std::string::ToString::to_string(#mapping_name)),
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
                                }))
                            ],
                            ..#crate_name::registry::MetaSchema::ANY
                        }))
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

    let (fn_schema_ref, fn_register) = if *args.inline {
        let fn_schema_ref =
            quote! { #crate_name::registry::MetaSchemaRef::Inline(Box::new(#meta)) };
        let fn_register = quote! { #(<#types as #crate_name::types::Type>::register(registry);)* };
        (fn_schema_ref, fn_register)
    } else {
        let fn_schema_ref =
            quote! { #crate_name::registry::MetaSchemaRef::Reference(#oai_typename) };
        let fn_register = quote! {
            registry.create_schema::<Self, _>(#oai_typename, |registry| {
                #(<#types as #crate_name::types::Type>::register(registry);)*
                #meta
            });
        };
        (fn_schema_ref, fn_register)
    };

    let expanded = if args.concretes.is_empty() {
        quote! {
            impl #impl_generics #crate_name::types::Type for #ident #ty_generics #where_clause {
                const IS_REQUIRED: bool = true;

                type RawValueType = Self;

                type RawElementValueType = Self;

                fn name() -> ::std::borrow::Cow<'static, str> {
                    ::std::convert::Into::into("object")
                }

                fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                    #fn_schema_ref
                }

                fn register(registry: &mut #crate_name::registry::Registry) {
                    #fn_register
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
        }
    } else {
        let mut code = Vec::new();

        code.push(quote! {
            impl #impl_generics #ident #ty_generics #where_clause {
                fn __internal_create_schema(registry: &mut #crate_name::registry::Registry) -> #crate_name::registry::MetaSchema
                where
                    Self: #crate_name::types::Type
                {
                    #(<#types as #crate_name::types::Type>::register(registry);)*
                    #meta
                }

                fn __internal_parse_from_json(value: ::std::option::Option<#crate_name::__private::serde_json::Value>) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>>
                where
                    Self: #crate_name::types::Type
                {
                    let value = value.unwrap_or_default();
                    #parse_from_json
                }

                fn __internal_to_json(&self) -> ::std::option::Option<#crate_name::__private::serde_json::Value>
                where
                    Self: #crate_name::types::Type
                {
                    match self {
                        #(#to_json),*
                    }
                }
            }
        });

        for concrete in &args.concretes {
            let oai_typename = &concrete.name;
            let params = &concrete.params.0;
            let concrete_type = quote! { #ident<#(#params),*> };

            let expanded = quote! {
                impl #crate_name::types::Type for #concrete_type {
                    const IS_REQUIRED: bool = true;

                    type RawValueType = Self;

                    type RawElementValueType = Self;

                    fn name() -> ::std::borrow::Cow<'static, str> {
                        ::std::convert::Into::into(#oai_typename)
                    }

                    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
                        ::std::option::Option::Some(self)
                    }

                    fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                        #crate_name::registry::MetaSchemaRef::Reference(#oai_typename)
                    }

                    fn register(registry: &mut #crate_name::registry::Registry) {
                        let mut meta = Self::__internal_create_schema(registry);
                        registry.create_schema::<Self, _>(#oai_typename, move |registry| meta);
                    }

                    fn raw_element_iter<'a>(&'a self) -> ::std::boxed::Box<dyn ::std::iter::Iterator<Item = &'a Self::RawElementValueType> + 'a> {
                        ::std::boxed::Box::new(::std::iter::IntoIterator::into_iter(self.as_raw_value()))
                    }
                }

                impl #crate_name::types::ParseFromJSON for #concrete_type {
                    fn parse_from_json(value: ::std::option::Option<#crate_name::__private::serde_json::Value>) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>> {
                        Self::__internal_parse_from_json(value)
                    }
                }

                impl #crate_name::types::ToJSON for #concrete_type {
                    fn to_json(&self) -> ::std::option::Option<#crate_name::__private::serde_json::Value> {
                        Self::__internal_to_json(self)
                    }
                }
            };
            code.push(expanded);
        }

        quote!(#(#code)*)
    };

    Ok(expanded)
}
