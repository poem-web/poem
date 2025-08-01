use darling::{FromMeta, util::SpannedValue};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Error, Expr, FnArg, ImplItem, ImplItemFn, ItemImpl, Pat, Path, ReturnType, Type, ext::IdentExt,
    visit_mut::VisitMut,
};

use crate::{
    common_args::{
        APIMethod, CodeSample, DefaultValue, ExampleValue, ExternalDocument, ExtraHeader,
    },
    error::GeneratorResult,
    parameter_style::ParameterStyle,
    utils::{
        RemoveLifetime, convert_oai_path, get_crate_name, get_description,
        get_summary_and_description, optional_literal, optional_literal_string, parse_oai_attrs,
        remove_description, remove_oai_attrs,
    },
    validators::Validators,
};

#[derive(FromMeta)]
pub(crate) struct APIArgs {
    #[darling(default)]
    internal: bool,
    #[darling(default, with = crate::utils::preserve_str_literal)]
    prefix_path: Option<Expr>,
    #[darling(default, multiple, rename = "tag")]
    common_tags: Vec<Path>,
    #[darling(default, multiple, rename = "response_header")]
    response_headers: Vec<ExtraHeader>,
    #[darling(default, multiple, rename = "request_header")]
    request_headers: Vec<ExtraHeader>,
    #[darling(default)]
    ignore_case: Option<bool>,
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
    #[darling(default, multiple, rename = "code_sample")]
    code_samples: Vec<CodeSample>,
    #[darling(default)]
    hidden: bool,
    #[darling(default)]
    ignore_case: Option<bool>,
}

#[derive(FromMeta, Default)]
struct APIOperationParam {
    // for parameter
    #[darling(default)]
    name: Option<String>,
    #[darling(default)]
    ignore_case: Option<bool>,
    #[darling(default)]
    deprecated: bool,
    #[darling(default)]
    default: Option<DefaultValue>,
    #[darling(default)]
    example: Option<ExampleValue>,
    #[darling(default)]
    validator: Option<Validators>,
    #[darling(default)]
    explode: Option<bool>,
    #[darling(default)]
    style: Option<ParameterStyle>,
    // for oauth
    #[darling(multiple, default, rename = "scope")]
    scopes: Vec<Path>,
}

struct Context {
    add_routes: Vec<TokenStream>,
    operations: Vec<(TokenStream, TokenStream)>,
    register_items: Vec<TokenStream>,
}

pub(crate) fn generate(args: APIArgs, mut item_impl: ItemImpl) -> GeneratorResult<TokenStream> {
    let crate_name = get_crate_name(args.internal);
    let ident = item_impl.self_ty.clone();
    let (impl_generics, _, where_clause) = item_impl.generics.split_for_impl();
    let mut ctx = Context {
        add_routes: Default::default(),
        operations: Default::default(),
        register_items: Default::default(),
    };

    for item in &mut item_impl.items {
        if let ImplItem::Fn(method) = item {
            if let Some(operation_args) = parse_oai_attrs::<APIOperation>(&method.attrs)? {
                if method.sig.asyncness.is_none() {
                    return Err(
                        Error::new_spanned(&method.sig.ident, "Must be asynchronous").into(),
                    );
                }

                generate_operation(&mut ctx, &crate_name, &args, operation_args, method)?;
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
                paths_map.entry(#path).or_default().push(#operation);
            });
        }
        paths
    };

    let expanded = quote! {
        #item_impl

        impl #impl_generics #crate_name::OpenApi for #ident #where_clause {
            fn meta() -> ::std::vec::Vec<#crate_name::registry::MetaApi> {
                ::std::vec![#crate_name::registry::MetaApi {
                    paths: {
                        use ::std::iter::{IntoIterator, Iterator};
                        let mut paths_map = #crate_name::__private::indexmap::IndexMap::<::std::string::String, ::std::vec::Vec<#crate_name::registry::MetaOperation>>::new();
                        #(#paths)*
                        paths_map.into_iter().map(|(path, operations)| #crate_name::registry::MetaPath {
                            path,
                            operations,
                        }).collect()
                    },
                }]
            }

            fn register(registry: &mut #crate_name::registry::Registry) {
                #(#register_items)*
            }

            fn add_routes(self, route_table: &mut ::std::collections::HashMap<::std::string::String, ::std::collections::HashMap<#crate_name::__private::poem::http::Method, #crate_name::__private::poem::endpoint::BoxEndpoint<'static>>>) {
                let api_obj = ::std::sync::Arc::new(self);
                #(#add_routes)*
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
    item_method: &mut ImplItemFn,
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
        code_samples,
        hidden,
        ignore_case,
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
    let prefix_path = &api_args.prefix_path;
    let (oai_path, new_path) = convert_oai_path(&path)?;
    let oai_path = prefix_path
        .as_ref()
        .map(|prefix| quote! { #crate_name::__private::join_path(#prefix, #oai_path) })
        .unwrap_or_else(|| quote! { ::std::string::ToString::to_string(#oai_path) });
    let new_path: TokenStream = prefix_path
        .as_ref()
        .map(|prefix| quote! { #crate_name::__private::join_path(#prefix, #new_path) })
        .unwrap_or_else(|| quote! { ::std::string::ToString::to_string(#new_path) });

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
    RemoveLifetime.visit_type_mut(&mut res_ty);

    let mut parse_args = Vec::new();
    let mut use_args = Vec::new();
    let mut request_meta = Vec::new();
    let mut params_meta = Vec::new();
    let mut security = Vec::new();

    let mut path_param_count = 0;

    for i in 1..item_method.sig.inputs.len() {
        let arg = &mut item_method.sig.inputs[i];
        let (arg_ident, mut arg_ty, operation_param, param_description) = match arg {
            FnArg::Typed(pat) => {
                let ident = match &*pat.pat {
                    Pat::Ident(ident) => ident,
                    Pat::TupleStruct(tuple_struct) => match tuple_struct.elems.first() {
                        Some(Pat::Ident(ident)) if tuple_struct.elems.len() == 1 => ident,
                        _ => {
                            return Err(Error::new_spanned(
                                tuple_struct,
                                "Only single element tuple structs are supported",
                            )
                            .into());
                        }
                    },
                    _ => return Err(Error::new_spanned(pat, "Invalid param definition").into()),
                };

                let ident = ident.ident.clone();
                let operation_param =
                    parse_oai_attrs::<APIOperationParam>(&pat.attrs)?.unwrap_or_default();
                let description = get_description(&pat.attrs)?;
                remove_oai_attrs(&mut pat.attrs);
                remove_description(&mut pat.attrs);
                (ident, pat.ty.clone(), operation_param, description)
            }
            FnArg::Receiver(_) => {
                return Err(Error::new_spanned(item_method, "Invalid method definition.").into());
            }
        };
        let is_path = match &*arg_ty {
            syn::Type::Path(syn::TypePath { qself: _, path }) => {
                path.segments.iter().any(|v| v.ident == "Path")
            }
            _ => false,
        };

        RemoveLifetime.visit_type_mut(&mut arg_ty);

        let pname = format_ident!("p{}", i);
        let param_name = operation_param
            .name
            .clone()
            .unwrap_or_else(|| arg_ident.unraw().to_string());
        let ignore_case = operation_param
            .ignore_case
            .or(ignore_case)
            .or(api_args.ignore_case)
            .unwrap_or(false);
        let extract_param_name = if is_path {
            let n = format!("param{path_param_count}");
            path_param_count += 1;
            n
        } else {
            param_name.clone()
        };
        use_args.push(pname.clone());

        if !hidden {
            // register arg type
            ctx.register_items.push(quote! {
                <#arg_ty as #crate_name::ApiExtractor>::register(registry);
            });
        }

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

        // example value for parameter
        let example_value = match &operation_param.example {
            Some(ExampleValue::Default) => {
                quote!(::std::option::Option::Some(<<#arg_ty as #crate_name::ApiExtractor>::ParamType as std::default::Default>::default))
            }
            Some(ExampleValue::Function(func_name)) => {
                quote!(::std::option::Option::Some(#func_name))
            }
            None => quote!(::std::option::Option::None),
        };

        let param_meta_example = match &operation_param.example {
            Some(ExampleValue::Default) => {
                quote!(#crate_name::types::ToJSON::to_json(&<<#arg_ty as #crate_name::ApiExtractor>::ParamType as std::default::Default>::default()))
            }
            Some(ExampleValue::Function(func_name)) => {
                quote!(#crate_name::types::ToJSON::to_json(&#func_name()))
            }
            None => quote!(::std::option::Option::None),
        };

        // validator
        let validator = operation_param.validator.clone().unwrap_or_default();
        let param_checker = validator.create_param_checker(crate_name, &res_ty, &param_name)?.map(|stream| {
            quote! {
                if <#arg_ty as #crate_name::ApiExtractor>::TYPES.contains(&#crate_name::ApiExtractorType::Parameter) {
                    if let ::std::option::Option::Some(value) = #crate_name::ApiExtractor::param_raw_type(&#pname) {
                        #stream
                    }
                }
            }
        }).unwrap_or_default();
        let validators_update_meta = validator.create_update_meta(crate_name)?;

        // do extract
        let explode = operation_param.explode.unwrap_or(true);

        let style = match &operation_param.style {
            Some(operation_param) => {
                quote!(::std::option::Option::Some(#crate_name::ParameterStyle::#operation_param))
            }
            None => quote!(::std::option::Option::None),
        };

        parse_args.push(quote! {
            let mut param_opts = #crate_name::ExtractParamOptions {
                name: #extract_param_name,
                ignore_case: #ignore_case,
                default_value: #default_value,
                example_value: #example_value,
                explode: #explode,
                style: #style,
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
            if <#arg_ty as #crate_name::ApiExtractor>::TYPES.contains(&#crate_name::ApiExtractorType::Parameter) {
                let mut original_schema = <#arg_ty as #crate_name::ApiExtractor>::param_schema_ref().unwrap();

                let mut patch_schema = {
                    let mut schema = #crate_name::registry::MetaSchema::ANY;
                    schema.default = #param_meta_default;
                    schema.example = #param_meta_example;
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
                    explode: #explode,
                    style: #style,
                };
                params.push(meta_param);
            }
        });

        // request object meta
        let param_desc = optional_literal(&param_description);
        request_meta.push(quote! {
            if <#arg_ty as #crate_name::ApiExtractor>::TYPES.contains(&#crate_name::ApiExtractorType::RequestObject) {
                request = <#arg_ty as #crate_name::ApiExtractor>::request_meta();
                if let ::std::option::Option::Some(ref mut request) = request.as_mut() {
                    if request.description.is_none() {
                        request.description = #param_desc;
                    }
                }
            }
        });

        // security meta
        let scopes = &operation_param.scopes;
        security.push(quote! {
            if <#arg_ty as #crate_name::ApiExtractor>::TYPES.contains(&#crate_name::ApiExtractorType::SecurityScheme) {
                for security_name in <#arg_ty as #crate_name::ApiExtractor>::security_schemes() {
                    security.push(<::std::collections::HashMap<&'static str, ::std::vec::Vec<&'static str>> as ::std::convert::From<_>>::from([
                        (security_name, ::std::vec![#(#crate_name::OAuthScopes::name(&#scopes)),*])
                    ]));
                }
                if <#arg_ty as #crate_name::ApiExtractor>::has_security_fallback() {
                    security.push(::std::collections::HashMap::<&'static str, ::std::vec::Vec<&'static str>>::new());
                }
            }
        });
    }

    if !hidden {
        if let Some(actual_type) = &actual_type {
            ctx.register_items
                .push(quote!(<#actual_type as #crate_name::ApiResponse>::register(registry);));
        } else {
            ctx.register_items
                .push(quote!(<#res_ty as #crate_name::ApiResponse>::register(registry);));
        }
    }

    let transform = transform.map(|transform| {
        quote! {
            let ep = #crate_name::__private::poem::EndpointExt::map_to_response(#transform(ep));
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
        let set_operation_id = operation_id.as_ref().map(|operation_id| {
            quote! {
                let ep = #crate_name::__private::poem::EndpointExt::after(ep, |mut res| async move {
                    let operator_id = #crate_name::OperationId(#operation_id);
                    match &mut res {
                        ::std::result::Result::Ok(resp) => resp.set_data(operator_id),
                        ::std::result::Result::Err(err) => err.set_data(operator_id),
                    }
                    res
                });
            }
        });

        ctx.add_routes.push(quote! {
            route_table.entry(#new_path)
                .or_default()
                .insert(#crate_name::__private::poem::http::Method::#http_method, {
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
                    #set_operation_id
                    #crate_name::__private::poem::EndpointExt::boxed(ep)
                });
        });
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
                explode: true,
                style: None,
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

    let code_samples = code_samples
        .iter()
        .map(|item| {
            let CodeSample {
                lang,
                label,
                source,
            } = item;
            let label = optional_literal(label);
            quote! {
                #crate_name::registry::MetaCodeSample {
                    lang: #lang,
                    label: #label,
                    source: #source,
                }
            }
        })
        .collect::<Vec<_>>();

    if !hidden {
        for method in &methods {
            let http_method = method.to_http_method();
            let meta_operation = quote! {
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
                    code_samples: ::std::vec![#(#code_samples),*],
                }
            };
            ctx.operations.push((oai_path.clone(), meta_operation));
        }
    }

    Ok(())
}
