use darling::{Error, FromMeta, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, ItemImpl, Pat};

use crate::utils::*;

#[derive(FromMeta, Default)]
pub(crate) struct ToolsArgs {}

#[derive(FromMeta, Default)]
pub(crate) struct ToolArgs {
    name: Option<String>,
}

#[derive(FromMeta, Default)]
pub(crate) struct ParamArgs {
    name: Option<String>,
}

pub(crate) fn generate(_args: ToolsArgs, mut item_impl: ItemImpl) -> Result<TokenStream> {
    let crate_name = get_crate_name();
    let ident = item_impl.self_ty.clone();
    let instructions = get_description(&item_impl.attrs).unwrap_or_default();
    let mut tools_descriptions = vec![];
    let mut contains = vec![];
    let mut req_types = vec![];
    let mut call = vec![];

    for item in &mut item_impl.items {
        if let ImplItem::Fn(method) = item {
            let tool_args = parse_mcp_attrs::<ToolArgs>(&method.attrs)?;
            remove_mcp_attrs(&mut method.attrs);

            let tool_name = match &tool_args.name {
                Some(name) => name.clone(),
                None => method.sig.ident.to_string(),
            };
            let tool_description = get_description(&method.attrs).unwrap_or_default();

            if method.sig.asyncness.is_none() {
                return Err(Error::custom("must be asynchronous").with_span(&method.sig.ident));
            }

            if method.sig.inputs.is_empty() {
                return Err(Error::custom("at least one `&self` receiver is required.")
                    .with_span(&method.sig.ident));
            }

            if !matches!(&method.sig.inputs[0], FnArg::Receiver(_)) {
                return Err(
                    Error::custom("the first parameter must be a `&self` receiver.")
                        .with_span(&method.sig.inputs[0]),
                );
            }

            let request_type = format_ident!("{}_Request", method.sig.ident);
            let mut args = vec![];
            let mut arg_names = vec![];

            for arg in method.sig.inputs.iter_mut().skip(1) {
                let FnArg::Typed(pat) = arg else {
                    unreachable!()
                };
                let Pat::Ident(ident) = &mut *pat.pat else {
                    return Err(Error::custom("expected ident").with_span(&pat.pat));
                };

                let param_args = parse_mcp_attrs::<ParamArgs>(&pat.attrs)?;
                let param_name = match &param_args.name {
                    Some(name) => quote!(#name),
                    None => quote!(#ident),
                };
                let param_desc = get_description(&pat.attrs).map(|desc| quote!( #[doc = #desc]));
                remove_description(&mut pat.attrs);
                let param_ty = &pat.ty;
                args.push(quote! {
                    #param_desc
                    #param_name: #param_ty,
                });
                arg_names.push(quote! {
                    #param_name
                });
            }

            tools_descriptions.push(quote! {
                #crate_name::protocol::tool::Tool {
                    name: #tool_name,
                    description: #tool_description,
                    input_schema: {
                        let schema = schemars::r#gen::SchemaGenerator::default().into_root_schema_for::<#request_type>();
                        #crate_name::private::serde_json::to_value(schema).expect("serialize schema")
                    },
                },
            });

            contains.push(quote! {
                #tool_name => true,
            });

            req_types.push(quote! {
                #[derive(serde::Deserialize, schemars::JsonSchema)]
                #[allow(non_camel_case_types)]
                struct #request_type {
                    #(#args)*
                }
            });

            let method_ident = &method.sig.ident;

            call.push(quote! {
                #tool_name => {
                    let #request_type { #(#arg_names),* } = #crate_name::private::serde_json::from_value(arguments.clone())
                        .map_err(|e| #crate_name::protocol::rpc::RpcError::invalid_params(format!("invalid parameters: {}", e)))?;
                    let response = self.#method_ident(#(#arg_names),*).await;
                    Ok(#crate_name::tool::IntoToolResponse::into_tool_response(response))
                }
            });
        }
    }

    Ok(quote! {
        #item_impl

        #(#req_types)*

        impl #crate_name::tool::Tools for #ident {
            fn instructions() -> &'static ::std::primitive::str {
                #instructions
            }

            fn list() -> ::std::vec::Vec<#crate_name::protocol::tool::Tool> {
                ::std::vec![#(#tools_descriptions)*]
            }

            async fn call(
                &mut self,
                name: &::std::primitive::str,
                arguments: #crate_name::private::serde_json::Value,
            ) -> ::std::result::Result<#crate_name::protocol::tool::ToolsCallResponse, #crate_name::protocol::rpc::RpcError> {
                match name {
                    #(#call)*
                    _ => Err(#crate_name::protocol::rpc::RpcError::method_not_found(format!("method not found: {}", name))),
                }
            }
        }
    })
}
