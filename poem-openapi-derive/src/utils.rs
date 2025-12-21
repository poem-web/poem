use std::collections::HashSet;

use darling::{FromMeta, util::SpannedValue};
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprLit, ExprMacro, GenericParam, Generics, Lifetime, Lit, LitStr,
    Meta, Result, visit_mut, visit_mut::VisitMut,
};

use crate::error::GeneratorResult;

/// Represents a documentation fragment that may be a literal string or a macro expression.
#[derive(Clone)]
pub(crate) enum DocFragment {
    /// A literal string known at proc-macro time
    Literal(String),
    /// A macro expression (like include_str!) that will be evaluated at compile time
    Macro(TokenStream),
}

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

/// Extracts documentation from attributes, returning a string if all doc fragments are literals.
///
/// This function only handles literal doc comments (e.g., `/// comment` or `#[doc = "string"]`).
/// Macro expressions like `#[doc = include_str!(...)]` are silently ignored.
/// For full macro support, use `get_description_token`.
pub(crate) fn get_description(attrs: &[Attribute]) -> Result<Option<String>> {
    let mut full_docs = String::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(doc), ..
                }) = &nv.value
                {
                    let doc = doc.value();
                    // Only trim trailing whitespace to preserve indentation for code blocks.
                    let doc_str = doc.trim_end();
                    if !full_docs.is_empty() {
                        full_docs += "\n";
                    }
                    // Doc comments typically have a leading space after `///`.
                    // Strip exactly one leading space if present, preserving any additional
                    // indentation for code blocks and other formatted content.
                    let doc_str = doc_str.strip_prefix(' ').unwrap_or(doc_str);
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

/// Extracts documentation from attributes, supporting both literal strings and macro expressions.
///
/// This function handles:
/// - Literal doc comments: `/// comment` or `#[doc = "string"]`
/// - Macro expressions: `#[doc = include_str!("path")]`
///
/// Returns a `TokenStream` that evaluates to `Option<&'static str>` at compile time.
/// When macro expressions are present, the result uses `concat!` to combine all fragments.
pub(crate) fn get_description_token(attrs: &[Attribute]) -> Result<Option<TokenStream>> {
    let mut fragments: Vec<DocFragment> = Vec::new();
    let mut current_literal = String::new();
    let mut has_macros = false;

    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                match &nv.value {
                    // Case 1: Regular string literal doc comments (/// or #[doc = "..."])
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(doc), ..
                    }) => {
                        let doc = doc.value();
                        // Only trim trailing whitespace to preserve indentation for code blocks.
                        let doc_str = doc.trim_end();
                        if !current_literal.is_empty() {
                            current_literal.push('\n');
                        }
                        // Doc comments typically have a leading space after `///`.
                        // Strip exactly one leading space if present.
                        let doc_str = doc_str.strip_prefix(' ').unwrap_or(doc_str);
                        current_literal.push_str(doc_str);
                    }
                    // Case 2: Macro expressions like include_str!(...)
                    Expr::Macro(ExprMacro { mac, .. }) => {
                        has_macros = true;
                        // Flush any accumulated literal docs first
                        if !current_literal.is_empty() {
                            current_literal.push('\n');
                            fragments
                                .push(DocFragment::Literal(std::mem::take(&mut current_literal)));
                        }
                        // Keep the macro as-is - it will be evaluated at compile time
                        fragments.push(DocFragment::Macro(quote!(#mac)));
                    }
                    _ => {}
                }
            }
        }
    }

    // Flush remaining literal docs
    if !current_literal.is_empty() {
        fragments.push(DocFragment::Literal(current_literal));
    }

    if fragments.is_empty() {
        return Ok(None);
    }

    // Optimization: if we only have literals and no macros, use a simple string
    if !has_macros {
        let combined: String = fragments
            .iter()
            .filter_map(|f| match f {
                DocFragment::Literal(s) => Some(s.as_str()),
                DocFragment::Macro(_) => None,
            })
            .collect::<Vec<_>>()
            .join("");
        return Ok(Some(quote!(::std::option::Option::Some(#combined))));
    }

    // Combine all parts using concat! for compile-time evaluation
    let concat_parts: Vec<TokenStream> = fragments
        .into_iter()
        .map(|fragment| match fragment {
            DocFragment::Literal(s) => {
                let lit = LitStr::new(&s, Span::call_site());
                quote!(#lit)
            }
            DocFragment::Macro(mac) => mac,
        })
        .collect();

    // Use concat! to combine at compile time
    Ok(Some(
        quote!(::std::option::Option::Some(::std::concat!(#(#concat_parts),*))),
    ))
}

/// Wraps a TokenStream description in a call to `.map(ToString::to_string)`.
#[allow(dead_code)] // May be useful for future use in other derive macros
pub(crate) fn description_to_string(desc: Option<TokenStream>) -> TokenStream {
    match desc {
        Some(ts) => quote!(#ts.map(::std::string::ToString::to_string)),
        None => quote!(::std::option::Option::None),
    }
}

pub(crate) fn remove_description(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("doc"));
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

/// Converts an `Option<TokenStream>` (from `get_description_token`) into a TokenStream
/// that evaluates to `Option<&'static str>`.
///
/// If `None`, returns `::std::option::Option::None`.
/// If `Some(ts)`, returns the TokenStream as-is (it already wraps in Option::Some).
pub(crate) fn optional_literal_token(ts: Option<TokenStream>) -> TokenStream {
    match ts {
        Some(ts) => ts,
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
        .find(|(_, a)| a.path().is_ident("oai"))
    {
        attrs.remove(idx);
    }
}

pub(crate) fn parse_oai_attrs<T: FromMeta>(attrs: &[Attribute]) -> GeneratorResult<Option<T>> {
    for attr in attrs {
        if attr.path().is_ident("oai") {
            return Ok(Some(T::from_meta(&attr.meta)?));
        }
    }
    Ok(None)
}

pub(crate) fn convert_oai_path(path: &SpannedValue<String>) -> Result<(String, String)> {
    if !path.starts_with('/') {
        return Err(Error::new(path.span(), "The path must start with '/'."));
    }

    let mut oai_path = String::new();
    let mut new_path = String::new();
    let mut vars = HashSet::new();
    let mut param_count = 0;

    for s in path.split('/') {
        if s.is_empty() {
            continue;
        }

        if let Some(var) = s.strip_prefix(':') {
            oai_path.push_str("/{");
            oai_path.push_str(var);
            oai_path.push('}');

            new_path.push_str("/:");
            new_path.push_str(&format!("param{param_count}"));
            param_count += 1;

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

    if oai_path.is_empty() {
        oai_path += "/";
    }

    if new_path.is_empty() {
        new_path += "/";
    }

    Ok((oai_path, new_path))
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

            name.push('_');
            name.push_str(&<#first as #crate_name::types::Type>::name());
            #(
                name.push_str("_");
                name.push_str(&<#tail as #crate_name::types::Type>::name());
            )*

            name
        })
    }
}

pub(crate) fn preserve_str_literal(meta: &Meta) -> darling::Result<Option<Expr>> {
    match meta {
        Meta::Path(_) => Err(darling::Error::unsupported_format("path").with_span(meta)),
        Meta::List(_) => Err(darling::Error::unsupported_format("list").with_span(meta)),
        Meta::NameValue(nv) => Ok(Some(nv.value.clone())),
    }
}
