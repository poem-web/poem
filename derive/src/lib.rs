//! Macros for poem

#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod utils;

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, FnArg, ItemFn, Result};

/// Wrap an asynchronous function as an `Endpoint`.
///
/// # Attributes
///
/// method - Add a method guard.
/// host=`"host name"` - Add a host guard.
/// header(`"header name"`, `"header value`) - Add a header value guard.
///
/// # Example
///
/// ```ignore
/// #[handler(
///     method = "get",
///     host = "example.com",
///     header(name = "Custom-header", value = "true")
/// )]
/// async fn example() {
/// }
/// ```
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

#[derive(Debug, Copy, Clone, FromMeta)]
#[darling(rename_all = "lowercase")]
enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Connect,
    Patch,
    Trace,
}

impl Method {
    fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "get",
            Method::Post => "post",
            Method::Put => "put",
            Method::Delete => "delete",
            Method::Head => "head",
            Method::Options => "options",
            Method::Connect => "connect",
            Method::Patch => "patch",
            Method::Trace => "trace",
        }
    }
}

#[derive(FromMeta, Default)]
struct Header {
    name: String,
    value: String,
}

#[derive(FromMeta, Default)]
#[darling(default)]
struct HandlerArgs {
    internal: bool,
    method: Option<Method>,
    host: Option<String>,
    #[darling(multiple, rename = "header")]
    headers: Vec<Header>,
}

fn generate_handler(args: HandlerArgs, input: TokenStream) -> Result<TokenStream> {
    let crate_name = utils::get_crate_name(args.internal);
    let item_fn = syn::parse2::<ItemFn>(input)?;
    let vis = &item_fn.vis;
    let ident = &item_fn.sig.ident;
    let call_await = if item_fn.sig.asyncness.is_some() {
        Some(quote::quote!(.await))
    } else {
        None
    };
    let mut guards = Vec::new();

    if let Some(method) = args.method {
        let method = quote::format_ident!("{}", method.as_str());
        guards.push(quote::quote!(#crate_name::guard::#method()));
    }

    if let Some(host) = args.host {
        guards.push(quote::quote!(#crate_name::guard::host(#host)));
    }

    for header in args.headers {
        let Header { name, value } = header;
        guards.push(quote::quote!(#crate_name::guard::header(#name, #value)));
    }

    let guard = {
        let guards = guards.into_iter().map(|guard| {
            quote! {
                if !#crate_name::Guard::check(&#guard, &req) {
                    return false;
                }
            }
        });
        quote! {
            fn check(&self, req: &#crate_name::Request) -> bool {
                #(#guards)*
                true
            }
        }
    };

    let mut extractors = Vec::new();
    let mut args = Vec::new();
    for (idx, input) in item_fn.sig.inputs.clone().into_iter().enumerate() {
        if let FnArg::Typed(pat) = input {
            let ty = &pat.ty;
            let id = quote::format_ident!("p{}", idx);
            args.push(id.clone());
            extractors.push(quote! {
                let #id = match <#ty as #crate_name::FromRequest>::from_request(&req, &mut body).await {
                    Ok(value) => value,
                    Err(err) => return err.as_response(),
                };
            });
        }
    }

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        #vis struct #ident;

        #[#crate_name::async_trait]
        impl #crate_name::Endpoint for #ident {
            #guard

            async fn call(&self, mut req: #crate_name::Request) -> #crate_name::Response {
                let (req, mut body) = req.split_body();
                #(#extractors)*
                #item_fn
                #crate_name::IntoResponse::into_response(#ident(#(#args),*)#call_await)
            }
        }
    };

    Ok(expanded)
}
