//! Macros for poem

#![doc(html_favicon_url = "https://raw.githubusercontent.com/poem-web/poem/master/favicon.ico")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/poem-web/poem/master/logo.png")]
#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod utils;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, AttributeArgs, FnArg, GenericParam, ItemFn, Member, Meta, NestedMeta, Result,
};

/// Wrap an asynchronous function as an `Endpoint`.
///
/// # Example
///
/// ```ignore
/// #[handler]
/// async fn example() {
/// }
/// ```
#[proc_macro_attribute]
pub fn handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let args: AttributeArgs = parse_macro_input!(args as AttributeArgs);
    let mut internal = false;

    for arg in args {
        if matches!(arg,NestedMeta::Meta(Meta::Path(p)) if p.is_ident("internal")) {
            internal = true;
        }
    }

    match generate_handler(internal, input) {
        Ok(stream) => stream,
        Err(err) => err.into_compile_error().into(),
    }
}

fn generate_handler(internal: bool, input: TokenStream) -> Result<TokenStream> {
    let crate_name = utils::get_crate_name(internal);
    let item_fn = syn::parse::<ItemFn>(input)?;
    let (impl_generics, type_generics, where_clause) = item_fn.sig.generics.split_for_impl();
    let vis = &item_fn.vis;
    let docs = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident("doc"))
        .cloned()
        .collect::<Vec<_>>();
    let ident = &item_fn.sig.ident;
    let call_await = if item_fn.sig.asyncness.is_some() {
        Some(quote::quote!(.await))
    } else {
        None
    };

    let def_struct = if !item_fn.sig.generics.params.is_empty() {
        let members = item_fn
            .sig
            .generics
            .params
            .iter()
            .filter_map(|param| match param {
                GenericParam::Type(ty) => Some(ty),
                _ => None,
            })
            .enumerate()
            .map(|(idx, ty)| {
                let ty_ident = &ty.ident;
                let ident = format_ident!("_mark{}", idx);
                quote! { #ident: ::std::marker::PhantomData<#ty_ident> }
            });
        quote! {
            #[derive(Default)]
            #vis struct #ident #type_generics { #(#members),*}
        }
    } else {
        quote! { #vis struct #ident; }
    };

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
        #(#docs)*
        #[allow(non_camel_case_types)]
        #def_struct

        #[#crate_name::async_trait]
        impl #impl_generics #crate_name::Endpoint for #ident #type_generics #where_clause {
            type Output = #crate_name::Response;

            #[allow(unused_mut)]
            async fn call(&self, mut req: #crate_name::Request) -> #crate_name::Result<Self::Output> {
                let (req, mut body) = req.split();
                #(#extractors)*
                #item_fn
                let res = #ident(#(#args),*)#call_await;
                let res = #crate_name::error::IntoResult::into_result(res);
                std::result::Result::map(res, #crate_name::IntoResponse::into_response)
            }
        }
    };

    Ok(expanded.into())
}

#[doc(hidden)]
#[proc_macro]
pub fn generate_implement_middlewares(_: TokenStream) -> TokenStream {
    let mut impls = Vec::new();

    for i in 2..=16 {
        let idents = (0..i)
            .map(|i| format_ident!("T{}", i + 1))
            .collect::<Vec<_>>();
        let output_type = idents.last().unwrap();
        let first_ident = idents.first().unwrap();
        let mut where_clauses = vec![quote! { #first_ident: Middleware<E> }];
        let mut transforms = Vec::new();

        for k in 1..i {
            let prev_ident = &idents[k - 1];
            let current_ident = &idents[k];
            where_clauses.push(quote! { #current_ident: Middleware<#prev_ident::Output> });
        }

        for k in 0..i {
            let n = Member::from(k);
            transforms.push(quote! { let ep = self.#n.transform(ep); });
        }

        let expanded = quote! {
            impl<E, #(#idents),*> Middleware<E> for (#(#idents),*)
                where
                    E: Endpoint,
                    #(#where_clauses,)*
            {
                type Output = #output_type::Output;

                fn transform(&self, ep: E) -> Self::Output {
                    #(#transforms)*
                    ep
                }
            }
        };

        impls.push(expanded);
    }

    quote!(#(#impls)*).into()
}
