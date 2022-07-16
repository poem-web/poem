use darling::{util::SpannedValue, FromMeta};
use indexmap::IndexMap;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    ext::IdentExt, visit_mut::VisitMut, AttributeArgs, Error, FnArg, ImplItem, ImplItemMethod,
    ItemImpl, Pat, Path, ReturnType, Type,
};

use crate::{
    common_args::{APIMethod, DefaultValue, ExternalDocument, ExtraHeader},
    error::GeneratorResult,
    utils::{
        convert_oai_path, get_crate_name, get_description, get_summary_and_description,
        optional_literal, optional_literal_string, parse_oai_attrs, remove_description,
        remove_oai_attrs, RemoveLifetime,
    },
    validators::Validators,
};

#[derive(FromMeta)]
struct APIArgs {
    #[darling(default)]
    internal: bool,
    #[darling(default)]
    prefix_path: Option<SpannedValue<String>>,
    #[darling(default, multiple, rename = "tag")]
    common_tags: Vec<Path>,
    #[darling(default, multiple, rename = "response_header")]
    response_headers: Vec<ExtraHeader>,
    #[darling(default, multiple, rename = "request_header")]
    request_headers: Vec<ExtraHeader>,
}

#[derive(FromMeta)]
struct APIOperation {
    path: SpannedValue<String>,
    #[darling(default, multiple, rename = "method")]
    methods: Vec<SpannedValue<APIMethod>>,
    #[darling(default)]
    deprecated: bool,
    #[darling(default, multiple, rename = "tag")]
    tags: Vec<Path>,
    #[darling(default)]
    transform: Option<Ident>,
    #[darling(default)]
    operation_id: Option<String>,
    #[darling(default)]
    external_docs: Option<ExternalDocument>,
    #[darling(default, multiple, rename = "response_header")]
    response_headers: Vec<ExtraHeader>,
    #[darling(default, multiple, rename = "request_header")]
    request_headers: Vec<ExtraHeader>,
    #[darling(default)]
    actual_type: Option<Type>,
}

#[derive(FromMeta, Default)]
struct APIOperationParam {
    // for parameter
    #[darling(default)]
    name: Option<String>,
    #[darling(default)]
    deprecated: bool,
    #[darling(default)]
    default: Option<DefaultValue>,
    #[darling(default)]
    validator: Option<Validators>,

    // for oauth
    #[darling(multiple, default, rename = "scope")]
    scopes: Vec<Path>,
}

struct Context {
    add_routes: IndexMap<String, IndexMap<APIMethod, TokenStream>>,
    operations: IndexMap<String, Vec<TokenStream>>,
    register_items: Vec<TokenStream>,
}

pub(crate) fn generate(
    args: AttributeArgs,
    mut item_impl: ItemImpl,
) -> GeneratorResult<TokenStream> {
    let api_args = match APIArgs::from_list(&args) {
        Ok(args) => args,
        Err(err) => return Ok(err.write_errors()),
    };
    let crate_name = get_crate_name(api_args.internal);
    let ident = item_impl.self_ty.clone();
    let (impl_generics, _, where_clause) = item_impl.generics.split_for_impl();
    let mut ctx = Context {
        add_routes: Default::default(),
        operations: Default::default(),
        register_items: Default::default(),
    };

    for item in &mut item_impl.items {
        if let ImplItem::Method(method) = item {
            if let Some(operation_args) = parse_oai_attrs::<APIOperation>(&method.attrs)? {
                if method.sig.asyncness.is_none() {
                    return Err(
                        Error::new_spanned(&method.sig.ident, "Must be asynchronous").into(),
                    );
                }

                generate_operation(&mut ctx, &crate_name, &api_args, operation_args, method)?;
                remove_oai_attrs(&mut method.attrs);
            }
        }
    }

    let Context {
        add_routes,
        operations,
        register_items,
    } = ctx;

    let paths = {
        let mut paths = Vec::new();

        for (path, operation) in operations {
            paths.push(quote! {
                #crate_name::registry::MetaPath {
                    path: #path,
                    operations: ::std::vec![#(#operation),*],
                }
            });
        }
        paths
    };

    let routes = {
        let mut routes = Vec::new();

        for (path, add_route) in add_routes {
            let add_route = add_route.values();
            routes.push(quote! {
                at(#path, #crate_name::__private::poem::RouteMethod::new()#(.#add_route)*)
            });
        }

        routes
    };

    let expanded = quote! {
        #item_impl

        impl #impl_generics #crate_name::OpenApi for #ident #where_clause {
            fn meta() -> ::std::vec::Vec<#crate_name::registry::MetaApi> {
                ::std::vec![#crate_name::registry::MetaApi {
                    paths: ::std::vec![#(#paths),*],
                }]
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                #(#register_items)*
            }

            fn add_routes(self, route: #crate_name::__private::poem::Route) -> #crate_name::__private::poem::Route {
                let api_obj = ::std::sync::Arc::new(self);
                route #(.#routes)*
            }
        }
    };

    Ok(expanded)
}

fn generate_operation(
    ctx: &mut Context,
    crate_name: &TokenStream,
    api_args: &APIArgs,
    args: APIOperation,
    item_method: &mut ImplItemMethod,
) -> GeneratorResult<()> {
    let APIOperation {
        path,
        methods,
        deprecated,
        tags,
        transform,
        operation_id,
        external_docs,
        response_headers,
        request_headers,
        actual_type,
    } = args;
    if methods.is_empty() {
        return Err(Error::new_spanned(
            &item_method.sig.ident,
            "At least one HTTP method is required",
        )
        .into());
    }
    let fn_ident = &item_method.sig.ident;
    let (summary, description) = get_summary_and_description(&item_method.attrs)?;
    let summary = optional_literal(&summary);
    let description = optional_literal(&description);
    let tags = api_args.common_tags.iter().chain(&tags);

    let (oai_path, new_path) = convert_oai_path(&path, &api_args.prefix_path)?;

    if item_method.sig.inputs.is_empty() {
        return Err(Error::new_spanned(
            &item_method.sig.ident,
            "At least one `&self` receiver is required.",
        )
        .into());
    }

    if let FnArg::Receiver(receiver) = &item_method.sig.inputs[0] {
        if receiver.mutability.is_some() {
            return Err(Error::new_spanned(
                receiver,
                "The first parameter must be a `&self` receiver.",
            )
            .into());
        }
    } else {
        return Err(Error::new_spanned(
            &item_method.sig.inputs[0],
            "The first parameter must be a `&self` receiver.",
        )
        .into());
    }

    let mut res_ty = match &item_method.sig.output {
        ReturnType::Default => Box::new(syn::parse2(quote!(())).unwrap()),
        ReturnType::Type(_, ty) => ty.clone(),
    };
    RemoveLifetime.visit_type_mut(&mut *res_ty);

    let mut parse_args = Vec::new();
    let mut use_args = Vec::new();
    let mut request_meta = Vec::new();
    let mut params_meta = Vec::new();
    let mut security = Vec::new();

    for i in 1..item_method.sig.inputs.len() {
        let arg = &mut item_method.sig.inputs[i];
        let (arg_ident, mut arg_ty, operation_param, param_description) = match arg {
            FnArg::Typed(pat) => {
                if let Pat::Ident(ident) = &*pat.pat {
                    let ident = ident.ident.clone();
                    let operation_param =
                        parse_oai_attrs::<APIOperationParam>(&pat.attrs)?.unwrap_or_default();
                    let description = get_description(&pat.attrs)?;
                    remove_oai_attrs(&mut pat.attrs);
                    remove_description(&mut pat.attrs);
                    (ident, pat.ty.clone(), operation_param, description)
                } else {
                    return Err(Error::new_spanned(pat, "Invalid param definition.").into());
                }
            }
            FnArg::Receiver(_) => {
                return Err(Error::new_spanned(item_method, "Invalid method definition.").into());
            }
        };

        RemoveLifetime.visit_type_mut(&mut *arg_ty);

        let pname = format_ident!("p{}", i);
        let param_name = operation_param
            .name
            .clone()
            .unwrap_or_else(|| arg_ident.unraw().to_string());
        use_args.push(pname.clone());

        // register
        ctx.register_items.push(quote! {
            <#arg_ty as #crate_name::ApiExtractor>::register(registry);
        });

        // default value for parameter
        let default_value = match &operation_param.default {
            Some(DefaultValue::Default) => {
                quote!(::std::option::Option::Some(<<#arg_ty as #crate_name::ApiExtractor>::ParamType as std::default::Default>::default))
            }
            Some(DefaultValue::Function(func_name)) => {
                quote!(::std::option::Option::Some(#func_name))
            }
            None => quote!(::std::option::Option::None),
        };
        let has_default = operation_param.default.is_some();
        let param_meta_default = match &operation_param.default {
            Some(DefaultValue::Default) => {
                quote!(#crate_name::types::ToJSON::to_json(&<<#arg_ty as #crate_name::ApiExtractor>::ParamType as std::default::Default>::default()))
            }
            Some(DefaultValue::Function(func_name)) => {
                quote!(#crate_name::types::ToJSON::to_json(&#func_name()))
            }
            None => quote!(::std::option::Option::None),
        };

        // validator
        let validator = operation_param.validator.clone().unwrap_or_default();
        let param_checker = validator.create_param_checker(crate_name, &res_ty, &param_name)?.map(|stream| {
            quote! {
                if <#arg_ty as #crate_name::ApiExtractor>::TYPE == #crate_name::ApiExtractorType::Parameter {
                    if let ::std::option::Option::Some(value) = #crate_name::ApiExtractor::param_raw_type(&#pname) {
                        #stream
                    }
                }
            }
        }).unwrap_or_default();
        let validators_update_meta = validator.create_update_meta(crate_name)?;

        // do extract
        parse_args.push(quote! {
            let mut param_opts = #crate_name::ExtractParamOptions {
                name: #param_name,
                default_value: #default_value,
            };

            let #pname = match <#arg_ty as #crate_name::ApiExtractor>::from_request(&request, &mut body, param_opts).await {
                ::std::result::Result::Ok(value) => value,
                ::std::result::Result::Err(err) if <#res_ty as #crate_name::ApiResponse>::BAD_REQUEST_HANDLER => {
                    let res = <#res_ty as #crate_name::ApiResponse>::from_parse_request_error(err);
                    let res = #crate_name::__private::poem::error::IntoResult::into_result(res);
                    return ::std::result::Result::map(res, #crate_name::__private::poem::IntoResponse::into_response);
                }
                ::std::result::Result::Err(err) => return ::std::result::Result::Err(::std::convert::Into::into(err)),
            };
            #param_checker
        });

        // param meta
        let param_desc = optional_literal_string(&param_description);
        let deprecated = operation_param.deprecated;
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
                    required: <#arg_ty as #crate_name::ApiExtractor>::PARAM_IS_REQUIRED && !#has_default,
                    deprecated: #deprecated,
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

        // security meta
        let scopes = &operation_param.scopes;
        security.push(quote! {
            if <#arg_ty as #crate_name::ApiExtractor>::TYPE == #crate_name::ApiExtractorType::SecurityScheme {
                security = ::std::vec![<::std::collections::HashMap<&'static str, ::std::vec::Vec<&'static str>> as ::std::convert::From<_>>::from([
                    (<#arg_ty as #crate_name::ApiExtractor>::security_scheme().unwrap(), ::std::vec![#(#crate_name::OAuthScopes::name(&#scopes)),*])
                ])];
            }
        });
    }

    ctx.register_items
        .push(quote!(<#res_ty as #crate_name::ApiResponse>::register(registry);));

    let transform = transform.map(|transform| {
        quote! {
            let ep = #transform(ep);
        }
    });
    let update_content_type = match &actual_type {
        Some(actual_type) => quote!(
            resp.headers_mut().insert(#crate_name::__private::poem::http::header::CONTENT_TYPE,
                #crate_name::__private::poem::http::HeaderValue::from_static(<#actual_type as #crate_name::payload::Payload>::CONTENT_TYPE)
            );
        ),
        None => quote!(),
    };

    for method in &methods {
        let http_method = method.to_http_method();
        if ctx.add_routes.entry(new_path.clone()).or_default().insert(**method, quote! {
            method(#crate_name::__private::poem::http::Method::#http_method, {
                let api_obj = ::std::clone::Clone::clone(&api_obj);
                let ep = #crate_name::__private::poem::endpoint::make(move |request| {
                    let api_obj = ::std::clone::Clone::clone(&api_obj);
                    async move {
                        let (request, mut body) = request.split();
                        #(#parse_args)*
                        let res = api_obj.#fn_ident(#(#use_args),*).await;
                        let res = #crate_name::__private::poem::error::IntoResult::into_result(res);
                        match ::std::result::Result::map(res, #crate_name::__private::poem::IntoResponse::into_response) {
                            ::std::result::Result::Ok(mut resp) => {
                                #update_content_type
                                ::std::result::Result::Ok(resp)
                            }
                            ::std::result::Result::Err(err) => ::std::result::Result::Err(err),
                        }
                    }
                });
                #transform
                ep
            })
        }).is_some() {
            return Err(Error::new(method.span(), "duplicate method").into());
        }
    }

    let mut tag_names = Vec::new();
    for tag in tags {
        ctx.register_items
            .push(quote!(#crate_name::Tags::register(&#tag, registry);));
        tag_names.push(quote!(#crate_name::Tags::name(&#tag)));
    }
    let operation_id = optional_literal(&operation_id);
    let external_docs = match external_docs {
        Some(external_docs) => {
            let s = external_docs.to_token_stream(crate_name);
            quote!(::std::option::Option::Some(#s))
        }
        None => quote!(::std::option::Option::None),
    };

    // extra request headers
    let mut update_extra_request_headers = Vec::new();
    for header in api_args.request_headers.iter().chain(&request_headers) {
        let name = header.name.to_uppercase();
        let description = optional_literal_string(&header.description);
        let ty = match syn::parse_str::<Type>(&header.ty) {
            Ok(ty) => ty,
            Err(_) => return Err(Error::new(header.ty.span(), "Invalid type").into()),
        };
        let deprecated = header.deprecated;

        update_extra_request_headers.push(quote! {
            params.push(#crate_name::registry::MetaOperationParam {
                name: ::std::string::ToString::to_string(#name),
                schema: <#ty as #crate_name::types::Type>::schema_ref(),
                in_type: #crate_name::registry::MetaParamIn::Header,
                description: #description,
                required: <#ty as #crate_name::types::Type>::IS_REQUIRED,
                deprecated: #deprecated,
            });
        });
    }

    // extra response headers
    let mut update_extra_response_headers = Vec::new();
    for (idx, header) in api_args
        .response_headers
        .iter()
        .chain(&response_headers)
        .enumerate()
    {
        let name = header.name.to_uppercase();
        let description = optional_literal_string(&header.description);
        let ty = match syn::parse_str::<Type>(&header.ty) {
            Ok(ty) => ty,
            Err(_) => return Err(Error::new(header.ty.span(), "Invalid type").into()),
        };
        let deprecated = header.deprecated;

        update_extra_response_headers.push(quote! {
            for resp in &mut meta.responses {
                resp.headers.insert(#idx, #crate_name::registry::MetaHeader {
                    name: ::std::string::ToString::to_string(#name),
                    description: #description,
                    required: <#ty as #crate_name::types::Type>::IS_REQUIRED,
                    deprecated: #deprecated,
                    schema: <#ty as #crate_name::types::Type>::schema_ref(),
                });
            }
        });
    }

    let resp_meta = match &actual_type {
        Some(actual_type) => quote!(<#actual_type as #crate_name::ApiResponse>::meta()),
        None => quote!(<#res_ty as #crate_name::ApiResponse>::meta()),
    };

    for method in &methods {
        let http_method = method.to_http_method();
        ctx.operations
            .entry(oai_path.clone())
            .or_default()
            .push(quote! {
                #crate_name::registry::MetaOperation {
                    tags: ::std::vec![#(#tag_names),*],
                    method: #crate_name::__private::poem::http::Method::#http_method,
                    summary: #summary,
                    description: #description,
                    external_docs: #external_docs,
                    params: {
                        let mut params = ::std::vec::Vec::new();
                        #(#update_extra_request_headers)*
                        #(#params_meta)*
                        params
                    },
                    request: {
                        let mut request = ::std::option::Option::None;
                        #(#request_meta)*
                        request
                    },
                    responses: {
                        let mut meta = #resp_meta;
                        #(#update_extra_response_headers)*
                        meta
                    },
                    deprecated: #deprecated,
                    security: {
                        let mut security = ::std::vec![];
                        #(#security)*
                        security
                    },
                    operation_id: #operation_id,
                }
            });
    }

    Ok(())
}
