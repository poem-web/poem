//! Macros for poem

#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod utils;

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Error, FnArg, ItemFn, Result};

/// Wrap an asynchronous function as an `Endpoint`.
#[proc_macro_attribute]
pub fn handler(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = match HandlerArgs::from_list(&parse_macro_input!(args as AttributeArgs)) {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    match generate_handler(args, input.into()) {
        Ok(stream) => stream.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[derive(FromMeta, Default)]
#[darling(default)]
struct HandlerArgs {
    internal: bool,
}

fn generate_handler(args: HandlerArgs, input: TokenStream) -> Result<TokenStream> {
    let crate_name = utils::get_crate_name(args.internal);
    let item_fn = syn::parse2::<ItemFn>(input)?;
    let vis = &item_fn.vis;
    let ident = &item_fn.sig.ident;

    if item_fn.sig.asyncness.is_none() {
        return Err(Error::new_spanned(
            &item_fn,
            "the `async` keyword is missing from the function declaration",
        ));
    }

    let mut extractors = Vec::new();
    let mut args = Vec::new();
    for (idx, input) in item_fn.sig.inputs.clone().into_iter().enumerate() {
        if let FnArg::Typed(pat) = input {
            let ty = &pat.ty;
            let id = quote::format_ident!("p{}", idx);
            args.push(id.clone());
            extractors.push(quote! {
                let #id = <#ty as #crate_name::FromRequest>::from_request(&req, &mut body).await?;
            });
        }
    }

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        #vis struct #ident;

        #[#crate_name::async_trait]
        impl #crate_name::Endpoint for #ident {
            async fn call(&self, mut req: #crate_name::Request) -> #crate_name::Result<#crate_name::Response> {
                let (req, mut body) = req.split_body();
                #(#extractors)*
                #item_fn
                #crate_name::IntoResponse::into_response(#ident(#(#args),*).await)
            }
        }
    };

    Ok(expanded)
}
