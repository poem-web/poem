//! Macros for poem

#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod utils;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, FnArg, ItemFn, Result};

/// Wrap an asynchronous function as an `Endpoint`.
#[proc_macro_attribute]
pub fn handler(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match generate_handler(args.into(), input.into()) {
        Ok(stream) => stream.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

fn generate_handler(_args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let crate_name = utils::get_crate_name();
    let item_fn = syn::parse2::<ItemFn>(input)?;
    let vis = &item_fn.vis;
    let ident = &item_fn.sig.ident;

    if item_fn.sig.asyncness.is_none() {
        return Err(Error::new_spanned(&item_fn, "must be asynchronous"));
    }

    let mut extractors = Vec::new();
    let mut args = Vec::new();
    for input in &item_fn.sig.inputs {
        if let FnArg::Typed(pat) = input {
            let ty = &pat.ty;
            let pat = &pat.pat;
            args.push(pat);
            extractors.push(quote! { let #pat = <#ty as #crate_name::FromRequest>::from_request(&req, &mut body).await?; });
        }
    }

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        #vis struct #ident;

        #[async_trait::async_trait]
        impl #crate_name::Endpoint for #ident {
            async fn call(
                &self,
                mut req: #crate_name::Request,
            ) -> #crate_name::Result<#crate_name::Response> {
                let mut body = req.take_body().ok();
                #(#extractors)*
                #item_fn
                #crate_name::IntoResponse::into_response(#ident(#(#args),*).await)
            }
        }
    };

    Ok(expanded)
}
