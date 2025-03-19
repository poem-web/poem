use darling::{Error, FromMeta};
use proc_macro2::{Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{Attribute, Expr, ExprLit, Ident, Lit, Meta};

pub(crate) fn get_crate_name() -> TokenStream {
    let name = match crate_name("poem-mcpserver") {
        Ok(FoundCrate::Name(name)) => name,
        Ok(FoundCrate::Itself) | Err(_) => "poem_mcpserver".to_string(),
    };
    let name = Ident::new(&name, Span::call_site());
    quote!(#name)
}

pub(crate) fn get_description(attrs: &[Attribute]) -> Option<String> {
    let mut full_docs = String::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(doc), ..
                }) = &nv.value
                {
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
    if full_docs.is_empty() {
        None
    } else {
        Some(full_docs)
    }
}

pub(crate) fn parse_mcp_attrs<T>(attrs: &[Attribute]) -> Result<T, Error>
where
    T: FromMeta + Default,
{
    for attr in attrs {
        if attr.path().is_ident("mcp") {
            return Ok(T::from_meta(&attr.meta)?);
        }
    }
    Ok(T::default())
}

pub(crate) fn remove_mcp_attrs(attrs: &mut Vec<Attribute>) {
    if let Some((idx, _)) = attrs
        .iter()
        .enumerate()
        .find(|(_, a)| a.path().is_ident("mcp"))
    {
        attrs.remove(idx);
    }
}

pub(crate) fn remove_description(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("doc"));
}
