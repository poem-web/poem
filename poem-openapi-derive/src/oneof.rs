use darling::{
    ast::{Data, Fields},
    util::Ignored,
    FromDeriveInput, FromVariant,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DeriveInput, Error, Type};

use crate::{error::GeneratorResult, utils::get_crate_name};

#[derive(FromVariant)]
#[darling(attributes(oai), forward_attrs(doc))]
struct OneOfItem {
    ident: Ident,
    fields: Fields<Type>,

    #[darling(default)]
    mapping: Option<String>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct OneOfArgs {
    ident: Ident,
    data: Data<OneOfItem, Ignored>,

    #[darling(default)]
    internal: bool,
    property_name: String,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: OneOfArgs = OneOfArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let ident = &args.ident;
    let property_name = &args.property_name;

    let e = match &args.data {
        Data::Enum(e) => e,
        _ => return Err(Error::new_spanned(ident, "OneOf can only be applied to an enum.").into()),
    };

    let mut types = Vec::new();
    let mut from_json = Vec::new();
    let mut to_json = Vec::new();
    let mut names = Vec::new();
    let mut mapping = Vec::new();

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

                types.push(object_ty);
                from_json.push(quote! {
                    ::std::option::Option::Some(property_name) if property_name == #mapping_name => {
                        <#object_ty as #crate_name::types::ParseFromJSON>::parse_from_json(value).map(Self::#item_ident).map_err(#crate_name::types::ParseError::propagate)
                    }
                });
                to_json.push(quote! {
                    Self::#item_ident(obj) => {
                        let mut value = <#object_ty as #crate_name::types::ToJSON>::to_json(obj);
                        if let ::std::option::Option::Some(obj) = value.as_object_mut() {
                            obj.insert(::std::convert::Into::into(#property_name), ::std::convert::Into::into(#mapping_name));
                        }
                        value
                    }
                });
                names.push(quote!(#mapping_name));

                if variant.mapping.is_some() {
                    mapping.push(quote! {
                        (#mapping_name, format!("#/components/schemas/{}", <#object_ty as #crate_name::types::Type>::schema_ref().unwrap_reference()))
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

    let expanded = quote! {
        impl #crate_name::types::Type for #ident {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> ::std::borrow::Cow<'static, str> {
                ::std::convert::Into::into("object")
            }

            fn schema_ref() -> #crate_name::registry::MetaSchemaRef {
                #crate_name::registry::MetaSchemaRef::Inline(Box::new(#crate_name::registry::MetaSchema {
                    one_of: ::std::vec![#(<#types as #crate_name::types::Type>::schema_ref()),*],
                    properties: ::std::vec![(#property_name, #crate_name::registry::MetaSchemaRef::Inline(Box::new(#crate_name::registry::MetaSchema {
                        enum_items: ::std::vec![#(::std::convert::Into::into(#names)),*],
                        ..#crate_name::registry::MetaSchema::new("string")
                    })))],
                    discriminator: ::std::option::Option::Some(#crate_name::registry::MetaDiscriminatorObject {
                        property_name: #property_name,
                        mapping: ::std::vec![#(#mapping),*],
                    }),
                    ..#crate_name::registry::MetaSchema::new("object")
                }))
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                #(<#types as #crate_name::types::Type>::register(registry);)*
            }

            fn as_raw_value(&self) -> ::std::option::Option<&Self::RawValueType> {
                ::std::option::Option::Some(self)
            }

            fn raw_element_iter<'a>(&'a self) -> ::std::boxed::Box<dyn ::std::iter::Iterator<Item = &'a Self::RawElementValueType> + 'a> {
                ::std::boxed::Box::new(::std::iter::IntoIterator::into_iter(self.as_raw_value()))
            }
        }

        impl #crate_name::types::ParseFromJSON for #ident {
            fn parse_from_json(value: #crate_name::__private::serde_json::Value) -> ::std::result::Result<Self, #crate_name::types::ParseError<Self>> {
                match value.as_object().and_then(|obj| obj.get(#property_name)) {
                    #(#from_json,)*
                    _ => ::std::result::Result::Err(#crate_name::types::ParseError::expected_type(value)),
                }
            }
        }

        impl #crate_name::types::ToJSON for #ident {
            fn to_json(&self) -> #crate_name::__private::serde_json::Value {
                match self {
                    #(#to_json),*
                }
            }
        }

        impl #crate_name::__private::serde::Serialize for #ident {
            fn serialize<S: #crate_name::__private::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                #crate_name::types::ToJSON::to_json(self).serialize(serializer)
            }
        }

        impl<'de> #crate_name::__private::serde::Deserialize<'de> for #ident {
            fn deserialize<D: #crate_name::__private::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                let value: #crate_name::__private::serde_json::Value = #crate_name::__private::serde::de::Deserialize::deserialize(deserializer)?;
                #crate_name::types::ParseFromJSON::parse_from_json(value).map_err(|err| #crate_name::__private::serde::de::Error::custom(err.into_message()))
            }
        }
    };

    Ok(expanded)
}
