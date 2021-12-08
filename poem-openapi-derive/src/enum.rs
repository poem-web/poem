use darling::{
    ast::{Data, Fields},
    util::Ignored,
    FromDeriveInput, FromVariant,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ext::IdentExt, DeriveInput, Error};

use crate::{
    common_args::{RenameRule, RenameRuleExt, RenameTarget},
    error::GeneratorResult,
    utils::get_crate_name,
};

#[derive(FromVariant)]
#[darling(attributes(oai), forward_attrs(doc))]
struct EnumItem {
    ident: Ident,
    fields: Fields<Ignored>,

    #[darling(default)]
    rename: Option<String>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct EnumArgs {
    ident: Ident,
    data: Data<EnumItem, Ignored>,

    #[darling(default)]
    internal: bool,
    #[darling(default)]
    rename_all: Option<RenameRule>,
    #[darling(default)]
    rename: Option<String>,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: EnumArgs = EnumArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let ident = &args.ident;
    let oai_typename = args
        .rename
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
        let oai_item_name = variant.rename.clone().unwrap_or_else(|| {
            args.rename_all
                .rename(variant.ident.unraw().to_string(), RenameTarget::EnumItem)
        });

        enum_items.push(quote!(#crate_name::types::ToJSON::to_json(&#ident::#item_ident)));
        ident_to_item.push(quote!(#ident::#item_ident => #oai_item_name));
        item_to_ident
            .push(quote!(#oai_item_name => ::std::result::Result::Ok(#ident::#item_ident)));
    }

    let expanded = quote! {
        impl #crate_name::types::Type for #ident {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> ::std::borrow::Cow<'static, str> {
                ::std::convert::Into::into(#oai_typename)
            }

            fn as_raw_value(&self) -> ::std::option::Option<&Self::RawValueType> {
                ::std::option::Option::Some(self)
            }

            fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                #crate_name::registry::MetaSchemaRef::Reference(#oai_typename)
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                registry.create_schema::<Self, _>(#oai_typename, |registry| #crate_name::registry::MetaSchema {
                    enum_items: ::std::vec![#(#enum_items),*],
                    ..#crate_name::registry::MetaSchema::new("string")
                });
            }

            fn raw_element_iter<'a>(&'a self) -> ::std::boxed::Box<dyn ::std::iter::Iterator<Item = &'a Self::RawElementValueType> + 'a> {
                ::std::boxed::Box::new(::std::iter::IntoIterator::into_iter(self.as_raw_value()))
            }
        }

        impl #crate_name::types::ParseFromJSON for #ident {
            fn parse_from_json(value: #crate_name::__private::serde_json::Value) -> #crate_name::types::ParseResult<Self> {
                match &value {
                    #crate_name::__private::serde_json::Value::String(item) => match item.as_str() {
                        #(#item_to_ident,)*
                        _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value)),
                    }
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
                    _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_input()),
                }
            }
        }

        impl #crate_name::types::ToJSON for #ident {
            fn to_json(&self) -> #crate_name::__private::serde_json::Value {
                let name = match self {
                    #(#ident_to_item),*
                };
                #crate_name::__private::serde_json::Value::String(::std::string::ToString::to_string(name))
            }
        }

        #[#crate_name::__private::poem::async_trait]
        impl #crate_name::types::ParseFromMultipartField for #ident {
            async fn parse_from_multipart(field: ::std::option::Option<#crate_name::__private::poem::web::Field>) -> #crate_name::types::ParseResult<Self> {
                use poem_openapi::types::ParseFromParameter;
                match field {
                    ::std::option::Option::Some(field) => {
                        let s = field.text().await?;
                        Self::parse_from_parameter(::std::option::Option::Some(&s))
                    },
                    ::std::option::Option::None => ::std::result::Result::Err(#crate_name::types::ParseError::expected_input()),
                }
            }
        }
    };

    Ok(expanded)
}
