use darling::{Error, FromMeta, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ImplItem, ItemImpl, Pat};

use crate::utils::*;

#[derive(FromMeta, Default)]
pub(crate) struct PromptsArgs {}

#[derive(FromMeta, Default)]
pub(crate) struct PromptArgs {
    name: Option<String>,
}

#[derive(FromMeta, Default)]
pub(crate) struct PromptParamArgs {
    name: Option<String>,
    #[darling(default)]
    required: bool,
}

pub(crate) fn generate(_args: PromptsArgs, mut item_impl: ItemImpl) -> Result<TokenStream> {
    let crate_name = get_crate_name();
    let ident = item_impl.self_ty.clone();
    let mut prompts_descriptions = vec![];
    let mut get_branches = vec![];

    for item in &mut item_impl.items {
        if let ImplItem::Fn(method) = item {
            let prompt_args = parse_mcp_attrs::<PromptArgs>(&method.attrs)?;
            remove_mcp_attrs(&mut method.attrs);

            let prompt_name = match &prompt_args.name {
                Some(name) => name.clone(),
                None => method.sig.ident.to_string(),
            };
            let prompt_description = get_description(&method.attrs).unwrap_or_default();

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

            let mut prompt_arguments = vec![];
            let mut arg_extractions = vec![];
            let mut arg_names = vec![];
            let mut required_checks = vec![];

            for arg in method.sig.inputs.iter_mut().skip(1) {
                let FnArg::Typed(pat) = arg else {
                    unreachable!()
                };
                let Pat::Ident(ident) = &mut *pat.pat else {
                    return Err(Error::custom("expected ident").with_span(&pat.pat));
                };

                let param_args = parse_mcp_attrs::<PromptParamArgs>(&pat.attrs)?;
                remove_mcp_attrs(&mut pat.attrs);

                let param_name = match &param_args.name {
                    Some(name) => name.clone(),
                    None => ident.ident.to_string(),
                };
                let param_desc = get_description(&pat.attrs).unwrap_or_default();
                remove_description(&mut pat.attrs);
                let is_required = param_args.required;

                let arg_ident = &ident.ident;

                prompt_arguments.push(quote! {
                    #crate_name::protocol::prompts::PromptArgument {
                        name: #param_name,
                        description: #param_desc,
                        required: #is_required,
                    },
                });

                if is_required {
                    required_checks.push(quote! {
                        if !arguments.contains_key(#param_name) {
                            return ::std::result::Result::Err(
                                #crate_name::protocol::rpc::RpcError::invalid_params(
                                    format!("missing required argument: {}", #param_name)
                                )
                            );
                        }
                    });
                }

                arg_extractions.push(quote! {
                    let #arg_ident: ::std::option::Option<::std::string::String> = arguments.get(#param_name).cloned();
                });
                arg_names.push(quote! { #arg_ident });
            }

            let method_ident = &method.sig.ident;

            get_branches.push(quote! {
                #prompt_name => {
                    #(#required_checks)*
                    #(#arg_extractions)*
                    let response = self.#method_ident(#(#arg_names),*).await;
                    ::std::result::Result::Ok(#crate_name::prompts::IntoPromptResponse::into_prompt_response(response))
                }
            });

            prompts_descriptions.push(quote! {
                #crate_name::protocol::prompts::Prompt {
                    name: #prompt_name,
                    description: #prompt_description,
                    arguments: &[#(#prompt_arguments)*],
                },
            });
        }
    }

    Ok(quote! {
        #item_impl

        impl #crate_name::prompts::Prompts for #ident {
            fn list() -> ::std::vec::Vec<#crate_name::protocol::prompts::Prompt> {
                ::std::vec![#(#prompts_descriptions)*]
            }

            async fn get(
                &self,
                name: &::std::primitive::str,
                arguments: ::std::collections::HashMap<::std::string::String, ::std::string::String>,
            ) -> ::std::result::Result<#crate_name::protocol::prompts::PromptGetResponse, #crate_name::protocol::rpc::RpcError> {
                match name {
                    #(#get_branches)*
                    _ => ::std::result::Result::Err(#crate_name::protocol::rpc::RpcError::method_not_found(format!("prompt not found: {}", name))),
                }
            }
        }
    })
}
