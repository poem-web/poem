use proc_macro2::{Ident, TokenStream};
use prost_build::Service;
use quote::{format_ident, quote};
use syn::{Expr, Path, Type};

use crate::{config::GrpcConfig, utils::get_crate_name};

struct MethodInfo<'a> {
    path: &'a str,
    service_ident: &'a Ident,
    proxy_service_ident: &'a Ident,
    method_ident: &'a Ident,
    input_type: &'a Type,
    output_type: &'a Type,
    crate_name: &'a TokenStream,
}

pub(crate) fn generate(config: &GrpcConfig, service: &Service, buf: &mut String) {
    let service_ident = format_ident!("{}", &service.name);
    let server_ident = format_ident!("{}Server", &service.name);
    let mut trait_methods = Vec::new();
    let mut endpoints = Vec::new();
    let codec_list = config
        .codec_list
        .iter()
        .map(|path| {
            syn::parse_str::<Path>(path).unwrap_or_else(|_| panic!("invalid codec path: `{path}`"))
        })
        .collect::<Vec<_>>();
    let server_middlewares = config
        .server_middlewares
        .iter()
        .map(|expr| {
            syn::parse_str::<Expr>(expr)
                .unwrap_or_else(|_| panic!("invalid server middleware: `{expr}`"))
        })
        .collect::<Vec<_>>();
    let crate_name = get_crate_name(config.internal);
    let service_name = if !config.emit_package && !service.package.is_empty() {
        format!("{}.{}", service.package, service.proto_name)
    } else {
        service.proto_name.clone()
    };

    for method in &service.methods {
        let method_ident = format_ident!("{}", &method.name);
        let input_type = syn::parse_str::<Type>(&method.input_type).unwrap();
        let output_type = syn::parse_str::<Type>(&method.output_type).unwrap();
        let path = format!("/{}", method.proto_name);
        let proxy_service_ident = format_ident!("{}{}Service", service.name, method.name);
        let method_info = MethodInfo {
            path: &path,
            service_ident: &service_ident,
            proxy_service_ident: &proxy_service_ident,
            method_ident: &method_ident,
            input_type: &input_type,
            output_type: &output_type,
            crate_name: &crate_name,
        };

        match (method.client_streaming, method.server_streaming) {
            (false, false) => {
                trait_methods.push(quote! {
                    fn #method_ident(&self, request: #crate_name::Request<#input_type>) -> impl ::std::future::Future<Output = ::std::result::Result<#crate_name::Response<#output_type>, #crate_name::Status>> + Send;
                });
                endpoints.push(generate_unary(&codec_list, method_info));
            }
            (true, false) => {
                trait_methods.push(quote! {
                    fn #method_ident(&self, request: #crate_name::Request<#crate_name::Streaming<#input_type>>) -> impl ::std::future::Future<Output = ::std::result::Result<#crate_name::Response<#output_type>, #crate_name::Status>> + Send;
                });
                endpoints.push(generate_client_streaming(&codec_list, method_info));
            }
            (false, true) => {
                trait_methods.push(quote! {
                    fn #method_ident(&self, request: #crate_name::Request<#input_type>) -> impl ::std::future::Future<Output = ::std::result::Result<#crate_name::Response<#crate_name::Streaming<#output_type>>, #crate_name::Status>> + Send;
                });
                endpoints.push(generate_server_streaming(&codec_list, method_info));
            }
            (true, true) => {
                trait_methods.push(quote! {
                    fn #method_ident(&self, request: #crate_name::Request<#crate_name::Streaming<#input_type>>) -> impl ::std::future::Future<Output = ::std::result::Result<#crate_name::Response<#crate_name::Streaming<#output_type>>, #crate_name::Status>> + Send;
                });
                endpoints.push(generate_bidirectional_streaming(&codec_list, method_info));
            }
        }
    }

    let apply_middlewares = server_middlewares.iter().map(|expr| {
        quote! {
            let ep = ep.with(#expr);
        }
    });

    let token_stream = quote! {
        #[allow(unused_imports)]
        pub trait #service_ident: Send + Sync + 'static {
            #(#trait_methods)*
        }

        #[allow(unused_imports)]
        pub struct #server_ident<T> {
            inner: ::std::sync::Arc<T>,
            send_compressed: ::std::option::Option<#crate_name::CompressionEncoding>,
            accept_compressed: ::std::sync::Arc<[#crate_name::CompressionEncoding]>,
        }

        impl<T> ::std::clone::Clone for #server_ident<T> {
            #[inline]
            fn clone(&self) -> Self {
                Self {
                    inner: self.inner.clone(),
                    send_compressed: self.send_compressed,
                    accept_compressed: self.accept_compressed.clone(),
                }
            }
        }

        impl<T: #service_ident> #crate_name::Service for #server_ident<T> {
            const NAME: &'static str = #service_name;
        }


        #[allow(dead_code)]
        impl<T> #server_ident<T> {
            /// Create a new GRPC server
            pub fn new(service: T) -> Self {
                Self {
                    inner: ::std::sync::Arc::new(service),
                    send_compressed: ::std::option::Option::None,
                    accept_compressed: ::std::sync::Arc::new([]),
                }
            }

            /// Set the compression encoding for sending
            pub fn send_compressed(self, encoding: #crate_name::CompressionEncoding) -> Self {
                Self {
                    send_compressed: Some(encoding),
                    ..self
                }
            }

            /// Set the compression encodings for accepting
            pub fn accept_compressed(self, encodings: impl ::std::convert::Into<::std::sync::Arc<[#crate_name::CompressionEncoding]>>) -> Self {
                Self {
                    accept_compressed: encodings.into(),
                    ..self
                }
            }
        }

        impl<T: #service_ident> ::poem::IntoEndpoint for #server_ident<T> {
            type Endpoint = ::poem::endpoint::BoxEndpoint<'static, ::poem::Response>;

            #[allow(clippy::redundant_clone)]
            #[allow(clippy::let_and_return)]
            fn into_endpoint(self) -> Self::Endpoint {
                use ::poem::endpoint::EndpointExt;

                let mut route = ::poem::Route::new();

                #(#endpoints)*
                let ep = route.before(|req| async move {
                    if req.version() != ::poem::http::Version::HTTP_2 {
                        return Err(::poem::Error::from_status(::poem::http::StatusCode::HTTP_VERSION_NOT_SUPPORTED));
                    }
                    Ok(req)
                });
                #(#apply_middlewares)*
                ep.boxed()
            }
        }
    };

    buf.push_str(&prettyplease::unparse(&syn::parse2(token_stream).unwrap()));
}

fn generice_call_with_codec(
    crate_name: &TokenStream,
    codec_list: &[Path],
    call: TokenStream,
) -> TokenStream {
    let codec_call = codec_list
        .iter()
        .map(|codec| {
            quote! {
                {
                    let codec = #codec::default();
                    if #crate_name::codec::Codec::check_content_type(&codec, ct) {
                        return #call;
                    }
                }
            }
        })
        .collect::<Vec<_>>();
    let codec_call = if !codec_call.is_empty() {
        Some(quote! {
            if let Some(ct) = req.content_type() {
                #(#codec_call)*
            }
        })
    } else {
        None
    };

    quote! {
        #codec_call

        let codec = <#crate_name::codec::ProstCodec<_, _> as ::std::default::Default>::default();
        #call
    }
}

fn generate_unary(codec_list: &[Path], method_info: MethodInfo) -> TokenStream {
    let MethodInfo {
        path,
        service_ident,
        proxy_service_ident,
        method_ident,
        input_type,
        output_type,
        crate_name,
    } = method_info;

    let call = generice_call_with_codec(
        crate_name,
        codec_list,
        quote! {
            #crate_name::server::GrpcServer::new(codec, server.send_compressed, &server.accept_compressed).unary(#proxy_service_ident(server.inner.clone()), req).await
        },
    );

    quote! {
        #[allow(non_camel_case_types)]
        struct #proxy_service_ident<T>(::std::sync::Arc<T>);

        impl<T: #service_ident> #crate_name::service::UnaryService<#input_type> for #proxy_service_ident<T> {
            type Response = #output_type;

            async fn call(
                &self,
                request: #crate_name::Request<#input_type>,
            ) -> Result<#crate_name::Response<Self::Response>, #crate_name::Status> {
                self.0.#method_ident(request).await
            }
        }

        route = route.at(#path, ::poem::endpoint::make({
            let server = self.clone();
            move |req| {
                let server = server.clone();
                async move { #call }
            }
        }));
    }
}

fn generate_client_streaming(codec_list: &[Path], method_info: MethodInfo) -> TokenStream {
    let MethodInfo {
        path,
        service_ident,
        proxy_service_ident,
        method_ident,
        input_type,
        output_type,
        crate_name,
    } = method_info;

    let call = generice_call_with_codec(
        crate_name,
        codec_list,
        quote! {
            #crate_name::server::GrpcServer::new(codec, server.send_compressed, &server.accept_compressed).client_streaming(#proxy_service_ident(server.inner.clone()), req).await
        },
    );

    quote! {
        #[allow(non_camel_case_types)]
        struct #proxy_service_ident<T>(::std::sync::Arc<T>);

        impl<T: #service_ident> #crate_name::service::ClientStreamingService<#input_type> for #proxy_service_ident<T> {
            type Response = #output_type;

            async fn call(
                &self,
                request: #crate_name::Request<#crate_name::Streaming<#input_type>>,
            ) -> Result<#crate_name::Response<Self::Response>, #crate_name::Status> {
                self.0.#method_ident(request).await
            }
        }

        route = route.at(#path, ::poem::endpoint::make({
            let server = self.clone();
            move |req| {
                let server = server.clone();
                async move { #call }
            }
        }));
    }
}

fn generate_server_streaming(codec_list: &[Path], method_info: MethodInfo) -> TokenStream {
    let MethodInfo {
        path,
        service_ident,
        proxy_service_ident,
        method_ident,
        input_type,
        output_type,
        crate_name,
    } = method_info;

    let call = generice_call_with_codec(
        crate_name,
        codec_list,
        quote! {
            #crate_name::server::GrpcServer::new(codec, server.send_compressed, &server.accept_compressed).server_streaming(#proxy_service_ident(server.inner.clone()), req).await
        },
    );

    quote! {
        #[allow(non_camel_case_types)]
        struct #proxy_service_ident<T>(::std::sync::Arc<T>);

        impl<T: #service_ident> #crate_name::service::ServerStreamingService<#input_type> for #proxy_service_ident<T> {
            type Response = #output_type;

            async fn call(
                &self,
                request: #crate_name::Request<#input_type>,
            ) -> Result<#crate_name::Response<#crate_name::Streaming<Self::Response>>, #crate_name::Status> {
                self.0.#method_ident(request).await
            }
        }

        route = route.at(#path, ::poem::endpoint::make({
            let server = self.clone();
            move |req| {
                let server = server.clone();
                async move { #call }
            }
        }));
    }
}

fn generate_bidirectional_streaming(codec_list: &[Path], method_info: MethodInfo) -> TokenStream {
    let MethodInfo {
        path,
        service_ident,
        proxy_service_ident,
        method_ident,
        input_type,
        output_type,
        crate_name,
    } = method_info;

    let call = generice_call_with_codec(
        crate_name,
        codec_list,
        quote! {
            #crate_name::server::GrpcServer::new(codec, server.send_compressed, &server.accept_compressed).bidirectional_streaming(#proxy_service_ident(server.inner.clone()), req).await
        },
    );

    quote! {
        #[allow(non_camel_case_types)]
        struct #proxy_service_ident<T>(::std::sync::Arc<T>);

        impl<T: #service_ident> #crate_name::service::BidirectionalStreamingService<#input_type> for #proxy_service_ident<T> {
            type Response = #output_type;

            async fn call(
                &self,
                request: #crate_name::Request<#crate_name::Streaming<#input_type>>,
            ) -> Result<#crate_name::Response<#crate_name::Streaming<Self::Response>>, #crate_name::Status> {
                self.0.#method_ident(request).await
            }
        }

        route = route.at(#path, ::poem::endpoint::make({
            let server = self.clone();
            move |req| {
                let server = server.clone();
                async move { #call }
            }
        }));
    }
}
