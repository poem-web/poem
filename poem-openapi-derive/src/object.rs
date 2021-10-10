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
        ConcreteType, DefaultValue, MaximumValidator, MinimumValidator, RenameRule, RenameRuleExt,
        RenameTarget,
    },
    error::GeneratorResult,
    utils::{get_crate_name, get_summary_and_description, optional_literal},
    validators::HasValidators,
};

#[derive(FromField)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ObjectField {
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
    write_only: bool,
    #[darling(default)]
    read_only: bool,

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

impl_has_validators!(ObjectField);

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ObjectArgs {
    ident: Ident,
    generics: Generics,
    attrs: Vec<Attribute>,
    data: Data<Ignored, ObjectField>,

    #[darling(default)]
    internal: bool,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    rename_all: Option<RenameRule>,
    #[darling(default, multiple, rename = "concrete")]
    concretes: Vec<ConcreteType>,
    #[darling(default)]
    deprecated: bool,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: ObjectArgs = ObjectArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();
    let ident = &args.ident;
    let s = match &args.data {
        Data::Struct(s) => s,
        _ => {
            return Err(
                Error::new_spanned(ident, "Object can only be applied to an struct.").into(),
            )
        }
    };
    let oai_typename = args
        .rename
        .clone()
        .unwrap_or_else(|| RenameTarget::Type.rename(ident.to_string()));
    let (title, description) = get_summary_and_description(&args.attrs)?;
    let mut deserialize_fields = Vec::new();
    let mut serialize_fields = Vec::new();
    let mut fields = Vec::new();
    let mut meta_fields = Vec::new();
    let mut required_fields = Vec::new();

    for field in &s.fields {
        let field_ident = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        let read_only = field.read_only;
        let write_only = field.write_only;

        if field.skip {
            deserialize_fields.push(quote! {
                let #field_ident: #field_ty = ::std::default::Default::default();
            });
            fields.push(field_ident);
            continue;
        }

        if read_only && write_only {
            return Err(Error::new_spanned(
                field_ident,
                "The `write_only` and `read_only` attributes cannot be enabled at the same time.",
            )
            .into());
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
            .create_obj_field_checker(&crate_name, &field_name)?;
        let validators_update_meta = field.validators().create_update_meta(&crate_name)?;

        fields.push(field_ident);

        if read_only {
            deserialize_fields.push(quote! {
                #[allow(non_snake_case)]
                let #field_ident: #field_ty = Default::default();
            });
        } else {
            match &field.default {
                Some(default_value) => {
                    let default_value = match default_value {
                        DefaultValue::Default => {
                            quote!(<#field_ty as ::std::default::Default>::default())
                        }
                        DefaultValue::Function(func_name) => quote!(#func_name()),
                    };

                    deserialize_fields.push(quote! {
                    #[allow(non_snake_case)]
                    let #field_ident: #field_ty = {
                        match obj.get(#field_name).cloned().unwrap_or_default() {
                            #crate_name::serde_json::Value::Null => #default_value,
                            value => {
                                let value = #crate_name::types::ParseFromJSON::parse_from_json(value).map_err(#crate_name::types::ParseError::propagate)?;
                                #validators_checker
                                value
                            }
                        }
                    };
                });
                }
                _ => {
                    deserialize_fields.push(quote! {
                    #[allow(non_snake_case)]
                    let #field_ident: #field_ty = {
                        let value = #crate_name::types::ParseFromJSON::parse_from_json(obj.get(#field_name).cloned().unwrap_or_default())
                            .map_err(#crate_name::types::ParseError::propagate)?;
                        #validators_checker
                        value
                    };
                });
                }
            };
        }

        if write_only {
            serialize_fields.push(quote! {});
        } else {
            serialize_fields.push(quote! {
                let value = #crate_name::types::ToJSON::to_json(&self.#field_ident);
                object.insert(::std::string::ToString::to_string(#field_name), value);
            });
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
            <#field_ty>::register(registry);

            let mut schema_ref = <#field_ty as #crate_name::types::Type>::schema_ref();

            if let #crate_name::registry::MetaSchemaRef::Inline(schema) = &mut schema_ref {
                schema.default = #field_meta_default;
                schema.read_only = #read_only;
                schema.write_only = #write_only;

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

        required_fields.push(quote! {
            if <#field_ty>::IS_REQUIRED {
                fields.push(#field_name);
            }
        });
    }

    let title = optional_literal(&title);
    let description = optional_literal(&description);
    let deprecated = args.deprecated;
    let meta = quote! {
        #crate_name::registry::MetaSchema {
            title: #title,
            description: #description,
            required: {
                #[allow(unused_mut)]
                let mut fields = ::std::vec::Vec::new();
                #(#required_fields)*
                fields
            },
            properties: ::std::vec![#(#meta_fields),*],
            deprecated: #deprecated,
            ..#crate_name::registry::MetaSchema::new("object")
        }
    };

    let expanded = if args.concretes.is_empty() {
        quote! {
            impl #crate_name::types::Type for #ident {
                const NAME: #crate_name::types::TypeName = #crate_name::types::TypeName::Normal {
                    ty: #oai_typename,
                    format: ::std::option::Option::None,
                };
                const IS_REQUIRED: bool = true;

                type ValueType = Self;

                fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                    #crate_name::registry::MetaSchemaRef::Reference(#oai_typename)
                }

                fn register(registry: &mut #crate_name::registry::Registry) {
                    registry.create_schema(#oai_typename, |registry| #meta);
                }

                fn as_value(&self) -> ::std::option::Option<&Self> {
                    ::std::option::Option::Some(self)
                }
            }

            impl #crate_name::types::ParseFromJSON for #ident {
                fn parse_from_json(value: #crate_name::serde_json::Value) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>> {
                    match value {
                        #crate_name::serde_json::Value::Object(obj) => {
                            #(#deserialize_fields)*
                            ::std::result::Result::Ok(Self { #(#fields),* })
                        }
                        _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value)),
                    }
                }
            }

            impl #crate_name::types::ToJSON for #ident {
                fn to_json(&self) -> #crate_name::serde_json::Value {
                    let mut object = ::#crate_name::serde_json::Map::new();
                    #(#serialize_fields)*
                    #crate_name::serde_json::Value::Object(object)
                }
            }

            impl #crate_name::serde::Serialize for #ident {
                fn serialize<S: #crate_name::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                    #crate_name::types::ToJSON::to_json(self).serialize(serializer)
                }
            }

            impl<'de> #crate_name::serde::Deserialize<'de> for #ident {
                fn deserialize<D: #crate_name::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                    let value: #crate_name::serde_json::Value = #crate_name::serde::de::Deserialize::deserialize(deserializer)?;
                    #crate_name::types::ParseFromJSON::parse_from_json(value).map_err(|err| #crate_name::serde::de::Error::custom(err.into_message()))
                }
            }
        }
    } else {
        let mut code = Vec::new();

        code.push(quote! {
            impl #impl_generics #ident #ty_generics #where_clause {
                fn __internal_register(registry: &mut #crate_name::registry::Registry) where Self: #crate_name::types::Type {
                    registry.create_schema(Self::NAME.type_name(), |registry| #meta);
                }

                fn __internal_parse_from_json(value: #crate_name::serde_json::Value) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>> where Self: #crate_name::types::Type {
                    match value {
                        #crate_name::serde_json::Value::Object(obj) => {
                            #(#deserialize_fields)*
                            ::std::result::Result::Ok(Self { #(#fields),* })
                        }
                        _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value)),
                    }
                }

                fn __internal_to_json(&self) -> #crate_name::serde_json::Value where Self: #crate_name::types::Type {
                    let mut object = ::serde_json::Map::new();
                    #(#serialize_fields)*
                    #crate_name::serde_json::Value::Object(object)
                }
            }
        });

        for concrete in &args.concretes {
            let oai_typename = &concrete.name;
            let params = &concrete.params.0;
            let concrete_type = quote! { #ident<#(#params),*> };

            let expanded = quote! {
                impl #crate_name::types::Type for #concrete_type {
                    const NAME: #crate_name::types::TypeName = #crate_name::types::TypeName::Normal {
                        ty: #oai_typename,
                        format: ::std::option::Option::None,
                    };
                    const IS_REQUIRED: bool = true;

                    type ValueType = Self;

                    fn as_value(&self) -> Option<&Self> {
                        Some(self)
                    }

                    fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                        #crate_name::registry::MetaSchemaRef::Reference(#oai_typename)
                    }

                    fn register(registry: &mut #crate_name::registry::Registry) {
                        Self::__internal_register(registry);
                    }
                }

                impl #crate_name::types::ParseFromJSON for #concrete_type {
                    fn parse_from_json(value: #crate_name::serde_json::Value) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>> {
                        Self::__internal_parse_from_json(value)
                    }
                }

                impl #crate_name::types::ToJSON for #concrete_type {
                    fn to_json(&self) -> #crate_name::serde_json::Value {
                        Self::__internal_to_json(self)
                    }
                }

                impl #crate_name::serde::Serialize for #concrete_type {
                    fn serialize<S: #crate_name::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                        #crate_name::types::ToJSON::to_json(self).serialize(serializer)
                    }
                }

                impl<'de> #crate_name::serde::Deserialize<'de> for #concrete_type {
                    fn deserialize<D: #crate_name::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                        let value: #crate_name::serde_json::Value = #crate_name::serde::de::Deserialize::deserialize(deserializer)?;
                        #crate_name::types::ParseFromJSON::parse_from_json(value).map_err(|err| #crate_name::serde::de::Error::custom(err.into_message()))
                    }
                }
            };
            code.push(expanded);
        }

        quote!(#(#code)*)
    };

    Ok(expanded)
}
