use std::collections::HashSet;

use darling::{util::SpannedValue, FromMeta};
use indexmap::IndexMap;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    ext::IdentExt, visit_mut::VisitMut, AttributeArgs, Error, FnArg, ItemTrait, Pat, Path,
    ReturnType, TraitItem, TraitItemMethod,
};

use crate::{
    common_args::{APIMethod, DefaultValue, ExternalDocument},
    error::GeneratorResult,
    utils::{
        get_crate_name, get_description, get_summary_and_description, optional_literal,
        optional_literal_string, parse_oai_attrs, remove_description, remove_oai_attrs,
        RemoveLifetime,
    },
    validators::Validators,
};

#[derive(FromMeta)]
struct WebhookArgs {
    #[darling(default)]
    internal: bool,
    #[darling(default, multiple, rename = "tag")]
    common_tags: Vec<Path>,
}

#[derive(FromMeta)]
struct WebhookOperation {
    #[darling(default)]
    name: Option<String>,
    method: SpannedValue<APIMethod>,
    #[darling(default)]
    deprecated: bool,
    #[darling(default, multiple, rename = "tag")]
    tags: Vec<Path>,
    #[darling(default)]
    operation_id: Option<String>,
    #[darling(default)]
    external_docs: Option<ExternalDocument>,
}

#[derive(FromMeta, Default)]
struct WebHookOperationParam {
    #[darling(default)]
    name: Option<String>,
    #[darling(default)]
    deprecated: bool,
    #[darling(default)]
    default: Option<DefaultValue>,
    #[darling(default)]
    validator: Option<Validators>,
    #[darling(default)]
    explode: Option<bool>,
}

struct Context {
    operations: IndexMap<APIMethod, TokenStream>,
    names: HashSet<String>,
    register_items: Vec<TokenStream>,
}

pub(crate) fn generate(
    args: AttributeArgs,
    mut trait_impl: ItemTrait,
) -> GeneratorResult<TokenStream> {
    let webhook_args = match WebhookArgs::from_list(&args) {
        Ok(args) => args,
        Err(err) => return Ok(err.write_errors()),
    };
    let crate_name = get_crate_name(webhook_args.internal);
    let ident = trait_impl.ident.clone();
    let mut ctx = Context {
        operations: Default::default(),
        names: Default::default(),
        register_items: vec![],
    };

    for item in &mut trait_impl.items {
        if let TraitItem::Method(method) = item {
            if let Some(operation_args) = parse_oai_attrs::<WebhookOperation>(&method.attrs)? {
                if method.sig.asyncness.is_none() {
                    return Err(
                        Error::new_spanned(&method.sig.ident, "Must be asynchronous").into(),
                    );
                }

                generate_operation(&mut ctx, &crate_name, &webhook_args, operation_args, method)?;
                remove_oai_attrs(&mut method.attrs);
            }
        }
    }

    let Context {
        operations,
        register_items,
        ..
    } = ctx;

    let operations = operations.values();

    let expanded = quote! {
        #[#crate_name::__private::poem::async_trait]
        #trait_impl

        impl #crate_name::Webhook for &dyn #ident {
            fn meta() -> ::std::vec::Vec<#crate_name::registry::MetaWebhook> {
                ::std::vec![#(#operations),*]
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                #(#register_items)*
            }
        }
    };

    Ok(expanded)
}

fn generate_operation(
    ctx: &mut Context,
    crate_name: &TokenStream,
    webhook_args: &WebhookArgs,
    args: WebhookOperation,
    trait_method: &mut TraitItemMethod,
) -> GeneratorResult<()> {
    let WebhookOperation {
        name,
        method,
        deprecated,
        tags,
        operation_id,
        external_docs,
    } = args;
    let name = name.unwrap_or_else(|| trait_method.sig.ident.to_string());
    let http_method = method.to_http_method();
    let (summary, description) = get_summary_and_description(&trait_method.attrs)?;
    let summary = optional_literal(&summary);
    let description = optional_literal(&description);
    let external_docs = match external_docs {
        Some(external_docs) => {
            let s = external_docs.to_token_stream(crate_name);
            quote!(::std::option::Option::Some(#s))
        }
        None => quote!(::std::option::Option::None),
    };
    let tags = webhook_args.common_tags.iter().chain(&tags);

    if trait_method.sig.inputs.is_empty() {
        return Err(Error::new_spanned(
            &trait_method.sig.ident,
            "At least one `&self` receiver is required.",
        )
        .into());
    }

    if !matches!(trait_method.sig.inputs[0], FnArg::Receiver(_)) {
        return Err(Error::new_spanned(
            &trait_method.sig.inputs[0],
            "The first parameter must be a `&self` or `&mut self` receiver.",
        )
        .into());
    }

    let mut res_ty = match &trait_method.sig.output {
        ReturnType::Default => Box::new(syn::parse2(quote!(())).unwrap()),
        ReturnType::Type(_, ty) => ty.clone(),
    };
    RemoveLifetime.visit_type_mut(&mut *res_ty);

    let mut request_meta = Vec::new();
    let mut params_meta = Vec::new();

    for i in 1..trait_method.sig.inputs.len() {
        let arg = &mut trait_method.sig.inputs[i];
        let (arg_ident, mut arg_ty, operation_param, param_description) = match arg {
            FnArg::Typed(pat) => {
                if let Pat::Ident(ident) = &*pat.pat {
                    let ident = ident.ident.clone();
                    let operation_param =
                        parse_oai_attrs::<WebHookOperationParam>(&pat.attrs)?.unwrap_or_default();
                    let description = get_description(&pat.attrs)?;
                    remove_oai_attrs(&mut pat.attrs);
                    remove_description(&mut pat.attrs);
                    (ident, pat.ty.clone(), operation_param, description)
                } else {
                    return Err(Error::new_spanned(pat, "Invalid param definition.").into());
                }
            }
            FnArg::Receiver(_) => {
                return Err(Error::new_spanned(trait_method, "Invalid method definition.").into());
            }
        };

        RemoveLifetime.visit_type_mut(&mut *arg_ty);

        // register
        ctx.register_items.push(quote! {
            <#arg_ty as #crate_name::ApiExtractor>::register(registry);
        });

        // default value for parameter
        let param_meta_default = match &operation_param.default {
            Some(DefaultValue::Default) => {
                quote!(::std::option::Option::Some(#crate_name::types::ToJSON::to_json(&<#arg_ty as ::std::default::Default>::default())))
            }
            Some(DefaultValue::Function(func_name)) => {
                quote!(::std::option::Option::Some(#crate_name::types::ToJSON::to_json(&#func_name())))
            }
            None => quote!(::std::option::Option::None),
        };

        // validator
        let validator = operation_param.validator.clone().unwrap_or_default();
        let validators_update_meta = validator.create_update_meta(crate_name)?;

        // param meta
        let param_name = operation_param
            .name
            .clone()
            .unwrap_or_else(|| arg_ident.unraw().to_string());
        let param_desc = optional_literal_string(&param_description);
        let deprecated = operation_param.deprecated;
        let explode = operation_param.explode.unwrap_or(true);

        params_meta.push(quote! {
            if <#arg_ty as #crate_name::ApiExtractor>::TYPE == #crate_name::ApiExtractorType::Parameter {
                let mut original_schema = <#arg_ty as #crate_name::ApiExtractor>::param_schema_ref().unwrap();

                let mut patch_schema = {
                    let mut schema = #crate_name::registry::MetaSchema::ANY;
                    schema.default = #param_meta_default;
                    #validators_update_meta
                    schema
                };

                let meta_param = #crate_name::registry::MetaOperationParam {
                    name: ::std::string::ToString::to_string(#param_name),
                    schema: original_schema.merge(patch_schema),
                    in_type: <#arg_ty as #crate_name::ApiExtractor>::param_in().unwrap(),
                    description: #param_desc,
                    required: <#arg_ty as #crate_name::ApiExtractor>::PARAM_IS_REQUIRED,
                    deprecated: #deprecated,
                    explode: #explode,
                };
                params.push(meta_param);
            }
        });

        // request object meta
        request_meta.push(quote! {
            if <#arg_ty as #crate_name::ApiExtractor>::TYPE == #crate_name::ApiExtractorType::RequestObject {
                request = <#arg_ty as #crate_name::ApiExtractor>::request_meta();
            }
        });
    }

    ctx.register_items
        .push(quote!(<#res_ty as #crate_name::ApiResponse>::register(registry);));

    let mut tag_names = Vec::new();
    for tag in tags {
        ctx.register_items
            .push(quote!(#crate_name::Tags::register(&#tag, registry);));
        tag_names.push(quote!(#crate_name::Tags::name(&#tag)));
    }
    let operation_id = optional_literal(&operation_id);

    if ctx.names.contains(&name) {
        return Err(Error::new(method.span(), "duplicate name").into());
    }

    if ctx
        .operations
        .insert(
            *method,
            quote! {
                #crate_name::registry::MetaWebhook {
                    name: #name,
                    operation: #crate_name::registry::MetaOperation {
                        tags: ::std::vec![#(#tag_names),*],
                        method: #crate_name::__private::poem::http::Method::#http_method,
                        summary: #summary,
                        description: #description,
                        external_docs: #external_docs,
                        params:  {
                            let mut params = ::std::vec::Vec::new();
                            #(#params_meta)*
                            params
                        },
                        request: {
                            let mut request = ::std::option::Option::None;
                            #(#request_meta)*
                            request
                        },
                        responses: <#res_ty as #crate_name::ApiResponse>::meta(),
                        deprecated: #deprecated,
                        security: ::std::vec![],
                        operation_id: #operation_id,
                        code_samples: ::std::vec![],
                    }
                }
            },
        )
        .is_some()
    {
        return Err(Error::new(method.span(), "duplicate method").into());
    }

    ctx.names.insert(name);
    Ok(())
}
