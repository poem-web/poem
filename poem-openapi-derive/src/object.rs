use darling::{ast::Data, util::Ignored, FromDeriveInput, FromField};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ext::IdentExt, Attribute, DeriveInput, Error, GenericParam, Generics, Type};

use crate::{
    common_args::{ConcreteType, DefaultValue, RenameRule, RenameRuleExt, RenameTarget},
    error::GeneratorResult,
    utils::{get_crate_name, get_summary_and_description, optional_literal},
    validators::Validators,
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
    validator: Option<Validators>,
}

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
    inline: bool,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    rename_all: Option<RenameRule>,
    #[darling(default, multiple, rename = "concrete")]
    concretes: Vec<ConcreteType>,
    #[darling(default)]
    deprecated: bool,
    #[darling(default)]
    read_only_all: bool,
    #[darling(default)]
    write_only_all: bool,
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
            );
        }
    };
    let oai_typename = args
        .rename
        .clone()
        .unwrap_or_else(|| RenameTarget::Type.rename(ident.to_string()));
    let (title, description) = get_summary_and_description(&args.attrs)?;
    let mut deserialize_fields = Vec::new();
    let mut serialize_fields = Vec::new();
    let mut register_types = Vec::new();
    let mut fields = Vec::new();
    let mut meta_fields = Vec::new();
    let mut required_fields = Vec::new();

    if args.inline && !args.concretes.is_empty() {
        return Err(Error::new_spanned(
            ident,
            "Inline objects cannot have the `concretes` attribute.",
        )
        .into());
    }

    for field in &s.fields {
        let field_ident = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        let read_only = args.read_only_all || field.read_only;
        let write_only = args.write_only_all || field.write_only;

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
                "The `write_only` and `read_only` attributes cannot be enabled both.",
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
        let validators = field.validator.clone().unwrap_or_default();
        let validators_checker = validators.create_obj_field_checker(&crate_name, &field_name)?;
        let validators_update_meta = validators.create_update_meta(&crate_name)?;

        fields.push(field_ident);

        if read_only {
            deserialize_fields.push(quote! {
                #[allow(non_snake_case)]
                let #field_ident: #field_ty = {
                    if obj.contains_key(#field_name) {
                        return Err(#crate_name::types::ParseError::custom(format!("properties `{}` is read only.", #field_name)));
                    }
                    Default::default()
                };
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
                            #crate_name::__private::serde_json::Value::Null => #default_value,
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

        register_types.push(quote!(<#field_ty>::register(registry);));

        meta_fields.push(quote! {{
            let patch_schema = {
                let mut schema = #crate_name::registry::MetaSchema::ANY;
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
                schema
            };

            (#field_name, <#field_ty as #crate_name::types::Type>::schema_ref().merge(patch_schema))
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
        let mut de_impl_generics = args.generics.clone();
        de_impl_generics
            .params
            .insert(0, GenericParam::Lifetime(syn::parse_str("'de").unwrap()));
        let (de_impl_generics, _, _) = de_impl_generics.split_for_impl();

        let (fn_schema_ref, fn_register) = if args.inline {
            (
                quote!(#crate_name::registry::MetaSchemaRef::Inline(Box::new(#meta))),
                quote! {
                    #(#register_types)*
                },
            )
        } else {
            (
                quote!(#crate_name::registry::MetaSchemaRef::Reference(#oai_typename)),
                quote! {
                    #(#register_types)*
                    registry.create_schema::<Self, _>(#oai_typename, |registry| #meta)
                },
            )
        };

        quote! {
            impl #impl_generics #crate_name::types::Type for #ident #ty_generics #where_clause {
                const IS_REQUIRED: bool = true;

                type RawValueType = Self;

                fn name() -> ::std::borrow::Cow<'static, str> {
                    ::std::convert::Into::into(#oai_typename)
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
            }

            impl #impl_generics #crate_name::types::ParseFromJSON for #ident #ty_generics #where_clause {
                fn parse_from_json(value: #crate_name::__private::serde_json::Value) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>> {
                    match value {
                        #crate_name::__private::serde_json::Value::Object(obj) => {
                            #(#deserialize_fields)*
                            ::std::result::Result::Ok(Self { #(#fields),* })
                        }
                        _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value)),
                    }
                }
            }

            impl #impl_generics #crate_name::types::ToJSON for #ident #ty_generics #where_clause {
                fn to_json(&self) -> #crate_name::__private::serde_json::Value {
                    let mut object = ::#crate_name::__private::serde_json::Map::new();
                    #(#serialize_fields)*
                    #crate_name::__private::serde_json::Value::Object(object)
                }
            }

            impl #impl_generics #crate_name::__private::serde::Serialize for #ident #ty_generics #where_clause {
                fn serialize<S: #crate_name::__private::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                    #crate_name::types::ToJSON::to_json(self).serialize(serializer)
                }
            }

            impl #de_impl_generics #crate_name::__private::serde::Deserialize<'de> for #ident #ty_generics #where_clause {
                fn deserialize<D: #crate_name::__private::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                    let value: #crate_name::__private::serde_json::Value = #crate_name::__private::serde::de::Deserialize::deserialize(deserializer)?;
                    #crate_name::types::ParseFromJSON::parse_from_json(value).map_err(|err| #crate_name::__private::serde::de::Error::custom(err.into_message()))
                }
            }
        }
    } else {
        let mut code = Vec::new();

        code.push(quote! {
            impl #impl_generics #ident #ty_generics #where_clause {
                fn __internal_register(name: &'static str, registry: &mut #crate_name::registry::Registry) where Self: #crate_name::types::Type {
                    #(#register_types)*
                    registry.create_schema::<Self, _>(name, |registry| #meta);
                }

                fn __internal_parse_from_json(value: #crate_name::__private::serde_json::Value) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>> where Self: #crate_name::types::Type {
                    match value {
                        #crate_name::__private::serde_json::Value::Object(obj) => {
                            #(#deserialize_fields)*
                            ::std::result::Result::Ok(Self { #(#fields),* })
                        }
                        _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value)),
                    }
                }

                fn __internal_to_json(&self) -> #crate_name::__private::serde_json::Value where Self: #crate_name::types::Type {
                    let mut object = ::serde_json::Map::new();
                    #(#serialize_fields)*
                    #crate_name::__private::serde_json::Value::Object(object)
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
                        Self::__internal_register(#oai_typename, registry);
                    }
                }

                impl #crate_name::types::ParseFromJSON for #concrete_type {
                    fn parse_from_json(value: #crate_name::__private::serde_json::Value) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>> {
                        Self::__internal_parse_from_json(value)
                    }
                }

                impl #crate_name::types::ToJSON for #concrete_type {
                    fn to_json(&self) -> #crate_name::__private::serde_json::Value {
                        Self::__internal_to_json(self)
                    }
                }

                impl #crate_name::__private::serde::Serialize for #concrete_type {
                    fn serialize<S: #crate_name::__private::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                        #crate_name::types::ToJSON::to_json(self).serialize(serializer)
                    }
                }

                impl<'de> #crate_name::__private::serde::Deserialize<'de> for #concrete_type {
                    fn deserialize<D: #crate_name::__private::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                        let value: #crate_name::__private::serde_json::Value = #crate_name::__private::serde::de::Deserialize::deserialize(deserializer)?;
                        #crate_name::types::ParseFromJSON::parse_from_json(value).map_err(|err| #crate_name::__private::serde::de::Error::custom(err.into_message()))
                    }
                }
            };
            code.push(expanded);
        }

        quote!(#(#code)*)
    };

    Ok(expanded)
}
