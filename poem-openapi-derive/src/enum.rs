use darling::{
    ast::{Data, Fields},
    util::Ignored,
    FromDeriveInput, FromVariant,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ext::IdentExt, DeriveInput, Error};

use crate::{
    common_args::{DefaultValue, RenameRule, RenameRuleExt, RenameTarget},
    error::GeneratorResult,
    utils::get_crate_name,
};

#[derive(FromVariant)]
#[darling(attributes(oai), forward_attrs(doc))]
struct EnumItem {
    ident: Ident,
    fields: Fields<Ignored>,

    #[darling(default)]
    pub name: Option<String>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct EnumArgs {
    ident: Ident,
    data: Data<EnumItem, Ignored>,

    #[darling(default)]
    internal: bool,
    #[darling(default)]
    rename_items: Option<RenameRule>,
    #[darling(default)]
    name: Option<String>,
    #[darling(default)]
    default: Option<DefaultValue>,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: EnumArgs = EnumArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let ident = &args.ident;
    let oai_typename = args
        .name
        .clone()
        .unwrap_or_else(|| RenameTarget::Type.rename(ident.to_string()));
    let e = match &args.data {
        Data::Enum(e) => e,
        _ => return Err(Error::new_spanned(ident, "Enum can only be applied to an enum.").into()),
    };

    let mut enum_items = Vec::new();
    let mut ident_to_item = Vec::new();
    let mut item_to_ident = Vec::new();

    for variant in e {
        if !variant.fields.is_empty() {
            return Err(Error::new_spanned(
                &variant.ident,
                format!(
                    "Invalid enum variant {}.\nOpenAPI enums may only contain unit variants.",
                    variant.ident
                ),
            )
            .into());
        }

        let item_ident = &variant.ident;
        let oai_item_name = variant.name.clone().unwrap_or_else(|| {
            args.rename_items
                .rename(variant.ident.unraw().to_string(), RenameTarget::EnumItem)
        });

        enum_items.push(quote!(#crate_name::types::ToJSON::to_json(&#ident::#item_ident)));
        ident_to_item.push(quote!(#ident::#item_ident => #oai_item_name));
        item_to_ident
            .push(quote!(#oai_item_name => ::std::result::Result::Ok(#ident::#item_ident)));
    }

    let meta_default_value = match &args.default {
        Some(DefaultValue::Default) => {
            quote!(::std::option::Option::Some(
                #crate_name::types::ToJSON::to_json(&<Self as ::std::default::Default>::default())
            ))
        }
        Some(DefaultValue::Function(func_name)) => {
            quote!(::std::option::Option::Some(#crate_name::types::ToJSON::to_json(&#func_name())))
        }
        None => quote!(::std::option::Option::None),
    };

    let default_in_parse = match &args.default {
        Some(DefaultValue::Default) => {
            quote! { #crate_name::serde_json::Value::Null => ::std::result::Result::Ok(<Self as ::std::default::Default>::default()), }
        }
        Some(DefaultValue::Function(func_name)) => {
            quote! { #crate_name::serde_json::Value::Null => ::std::result::Result::Ok(#func_name()), }
        }
        None => quote! {},
    };

    let default_in_parse_str = match &args.default {
        Some(DefaultValue::Default) => {
            quote! { ::std::option::Option::None => ::std::result::Result::Ok(<Self as ::std::default::Default>::default()), }
        }
        Some(DefaultValue::Function(func_name)) => {
            quote! { ::std::option::Option::None => ::std::result::Result::Ok(#func_name()), }
        }
        None => {
            quote! {}
        }
    };
    let is_required = args.default.is_none();

    let expanded = quote! {
        impl #crate_name::types::Type for #ident {
            const NAME: #crate_name::types::TypeName = #crate_name::types::TypeName::Normal {
                ty: #oai_typename,
                format: ::std::option::Option::None,
            };
            const IS_REQUIRED: bool = #is_required;

            type ValueType = Self;

            fn as_value(&self) -> ::std::option::Option<&Self> {
                ::std::option::Option::Some(self)
            }

            fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                #crate_name::registry::MetaSchemaRef::Reference(#oai_typename)
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                registry.create_schema(#oai_typename, |registry| #crate_name::registry::MetaSchema {
                    enum_items: ::std::vec![#(#enum_items),*],
                    default: #meta_default_value,
                    ..#crate_name::registry::MetaSchema::new(#oai_typename)
                });
            }
        }

        impl #crate_name::types::ParseFromJSON for #ident {
            fn parse_from_json(value: #crate_name::serde_json::Value) -> #crate_name::types::ParseResult<Self> {
                match &value {
                    #crate_name::serde_json::Value::String(item) => match item.as_str() {
                        #(#item_to_ident,)*
                        _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value)),
                    }
                    #default_in_parse
                    _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value)),
                }
            }
        }

        impl #crate_name::types::ParseFromParameter for #ident {
            fn parse_from_parameter(value: ::std::option::Option<&str>) -> #crate_name::types::ParseResult<Self> {
                match value {
                    ::std::option::Option::Some(value) => match value {
                        #(#item_to_ident,)*
                        _ => ::std::result::Result::Err(#crate_name::types::ParseError::custom("Expect a valid enumeration value.")),
                    },
                    #default_in_parse_str
                    _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_input()),
                }
            }
        }

        impl #crate_name::types::ToJSON for #ident {
            fn to_json(&self) -> #crate_name::serde_json::Value {
                let name = match self {
                    #(#ident_to_item),*
                };
                #crate_name::serde_json::Value::String(::std::string::ToString::to_string(name))
            }
        }
    };

    Ok(expanded)
}
