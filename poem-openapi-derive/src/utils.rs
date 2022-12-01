use std::collections::HashSet;

use darling::{util::SpannedValue, FromMeta};
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{
    visit_mut, visit_mut::VisitMut, Attribute, Error, GenericParam, Generics, Lifetime, Lit, Meta,
    Result,
};

use crate::error::GeneratorResult;

pub(crate) fn get_crate_name(internal: bool) -> TokenStream {
    if internal {
        quote! { crate }
    } else {
        let name = match crate_name("poem-openapi") {
            Ok(FoundCrate::Name(name)) => name,
            Ok(FoundCrate::Itself) | Err(_) => "poem_openapi".to_string(),
        };
        let name = Ident::new(&name, Span::call_site());
        quote!(#name)
    }
}

pub(crate) fn get_description(attrs: &[Attribute]) -> Result<Option<String>> {
    let mut full_docs = String::new();
    for attr in attrs {
        if attr.path.is_ident("doc") {
            if let Meta::NameValue(nv) = attr.parse_meta()? {
                if let Lit::Str(doc) = nv.lit {
                    let doc = doc.value();
                    let doc_str = doc.trim();
                    if !full_docs.is_empty() {
                        full_docs += "\n";
                    }
                    full_docs += doc_str;
                }
            }
        }
    }
    Ok(if full_docs.is_empty() {
        None
    } else {
        Some(full_docs)
    })
}

pub(crate) fn remove_description(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path.is_ident("doc"));
}

pub(crate) fn get_summary_and_description(
    attrs: &[Attribute],
) -> Result<(Option<String>, Option<String>)> {
    let doc = get_description(attrs)?;

    match doc {
        Some(doc) => match doc.split_once("\n\n") {
            Some((summary, description)) => {
                Ok((Some(summary.to_string()), Some(description.to_string())))
            }
            None => Ok((Some(doc), None)),
        },
        None => Ok((None, None)),
    }
}

pub(crate) fn optional_literal(s: &Option<impl AsRef<str>>) -> TokenStream {
    match s {
        Some(s) => {
            let s = s.as_ref();
            quote!(::std::option::Option::Some(#s))
        }
        None => quote!(::std::option::Option::None),
    }
}

pub(crate) fn optional_literal_string(s: &Option<impl AsRef<str>>) -> TokenStream {
    match s {
        Some(s) => {
            let s = s.as_ref();
            quote!(::std::option::Option::Some(::std::string::ToString::to_string(#s)))
        }
        None => quote!(::std::option::Option::None),
    }
}

pub(crate) fn remove_oai_attrs(attrs: &mut Vec<Attribute>) {
    if let Some((idx, _)) = attrs
        .iter()
        .enumerate()
        .find(|(_, a)| a.path.is_ident("oai"))
    {
        attrs.remove(idx);
    }
}

pub(crate) fn parse_oai_attrs<T: FromMeta>(attrs: &[Attribute]) -> GeneratorResult<Option<T>> {
    for attr in attrs {
        if attr.path.is_ident("oai") {
            let meta = attr.parse_meta()?;
            return Ok(Some(T::from_meta(&meta)?));
        }
    }
    Ok(None)
}

pub(crate) fn convert_oai_path<'a, 'b: 'a>(
    path: &'a SpannedValue<String>,
    prefix_path: &'b Option<SpannedValue<String>>,
) -> Result<(String, String)> {
    if !path.starts_with('/') {
        return Err(Error::new(path.span(), "The path must start with '/'."));
    }

    let mut oai_path = String::new();
    let mut new_path = String::new();

    if let Some(prefix_path) = prefix_path {
        handle_path(prefix_path, &mut oai_path, &mut new_path)?;
    }

    handle_path(path, &mut oai_path, &mut new_path)?;

    if oai_path.is_empty() {
        oai_path += "/";
    }

    if new_path.is_empty() {
        new_path += "/";
    }

    Ok((oai_path, new_path))
}

#[allow(clippy::needless_lifetimes)]
fn handle_path<'a>(
    path: &'a SpannedValue<String>,
    oai_path: &mut String,
    new_path: &mut String,
) -> Result<()> {
    let mut vars = HashSet::new();

    for s in path.split('/') {
        if s.is_empty() {
            continue;
        }

        if let Some(var) = s.strip_prefix(':') {
            oai_path.push_str("/{");
            oai_path.push_str(var);
            oai_path.push('}');

            new_path.push_str("/:");
            new_path.push_str(var);

            if !vars.insert(var) {
                return Err(Error::new(
                    path.span(),
                    format!("Repeated path variable `{}`.", &s[1..]),
                ));
            }
        } else {
            oai_path.push('/');
            oai_path.push_str(s);

            new_path.push('/');
            new_path.push_str(s);
        }
    }
    Ok(())
}

pub(crate) struct RemoveLifetime;

impl VisitMut for RemoveLifetime {
    fn visit_lifetime_mut(&mut self, i: &mut Lifetime) {
        i.ident = Ident::new("_", Span::call_site());
        visit_mut::visit_lifetime_mut(self, i);
    }
}

pub(crate) fn create_object_name(
    crate_name: &TokenStream,
    name: &str,
    generics: &Generics,
) -> TokenStream {
    let types = generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(ty) => Some(&ty.ident),
            _ => None,
        })
        .collect::<Vec<_>>();

    if types.is_empty() {
        quote!({
            use ::std::convert::From;
            ::std::string::String::from(#name)
        })
    } else {
        let (first, tail) = types.split_first().unwrap();
        quote!({
            use ::std::convert::From;
            let mut name = ::std::string::String::from(#name);

            name.push('<');
            name.push_str(&<#first as #crate_name::types::Type>::name());
            #(
                name.push_str(", ");
                name.push_str(&<#tail as #crate_name::types::Type>::name());
            )*
            name.push('>');

            name
        })
    }
}
