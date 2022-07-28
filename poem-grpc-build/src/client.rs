use proc_macro2::{Ident, TokenStream};
use prost_build::Service;
use quote::{format_ident, quote};
use syn::Type;

use crate::{config::GrpcConfig, utils::get_crate_name};

pub(crate) fn generate(config: &GrpcConfig, service: &Service, buf: &mut String) {
    let client_ident = format_ident!("{}Client", &service.name);
    let crate_name = get_crate_name(config.internal);
    let mut methods = Vec::new();

    for method in &service.methods {
        let name = format_ident!("{}", method.name);
        let input_type = syn::parse_str::<Type>(&method.input_type).unwrap();
        let output_type = syn::parse_str::<Type>(&method.output_type).unwrap();
        let path = if !config.emit_package {
            format!(
                "/{}.{}/{}",
                service.package, service.proto_name, method.proto_name
            )
        } else {
            format!("/{}/{}", service.proto_name, method.proto_name)
        };

        match (method.client_streaming, method.server_streaming) {
            (false, false) => methods.push(generate_unary(
                &crate_name,
                &name,
                &path,
                &input_type,
                &output_type,
            )),
            (true, false) => methods.push(generate_client_streaming(
                &crate_name,
                &name,
                &path,
                &input_type,
                &output_type,
            )),
            (false, true) => methods.push(generate_server_streaming(
                &crate_name,
                &name,
                &path,
                &input_type,
                &output_type,
            )),
            (true, true) => methods.push(generate_bidirectional_streaming(
                &crate_name,
                &name,
                &path,
                &input_type,
                &output_type,
            )),
        }
    }

    let token_stream = quote! {
        #[allow(unused_imports)]
        pub struct #client_ident {
            cli: #crate_name::client::GrpcClient,
        }

        impl #client_ident {
            pub fn new(base_url: impl ::std::convert::Into<::std::string::String>) -> Self {
                Self {
                    cli: #crate_name::client::GrpcClient::new(base_url),
                }
            }

            #(
            #[allow(dead_code)]
            #methods
            )*
        }
    };

    buf.push_str(&prettyplease::unparse(&syn::parse2(token_stream).unwrap()));
}

fn generate_unary(
    crate_name: &TokenStream,
    name: &Ident,
    path: &str,
    input_type: &Type,
    output_type: &Type,
) -> TokenStream {
    quote! {
        pub async fn #name(&self, request: #crate_name::Request<#input_type>) -> ::std::result::Result<#crate_name::Response<#output_type>, #crate_name::Status> {
            let codec = <#crate_name::codec::ProstCodec<_, _> as ::std::default::Default>::default();
            self.cli.unary(#path, codec, request).await
        }
    }
}

fn generate_client_streaming(
    crate_name: &TokenStream,
    name: &Ident,
    path: &str,
    input_type: &Type,
    output_type: &Type,
) -> TokenStream {
    quote! {
        pub async fn #name(&self, request: #crate_name::Request<#crate_name::Streaming<#input_type>>) -> ::std::result::Result<#crate_name::Response<#output_type>, #crate_name::Status> {
            let codec = <#crate_name::codec::ProstCodec<_, _> as ::std::default::Default>::default();
            self.cli.client_streaming(#path, codec, request).await
        }
    }
}

fn generate_server_streaming(
    crate_name: &TokenStream,
    name: &Ident,
    path: &str,
    input_type: &Type,
    output_type: &Type,
) -> TokenStream {
    quote! {
        pub async fn #name(&self, request: #crate_name::Request<#input_type>) -> ::std::result::Result<#crate_name::Response<#crate_name::Streaming<#output_type>>, #crate_name::Status> {
            let codec = <#crate_name::codec::ProstCodec<_, _> as ::std::default::Default>::default();
            self.cli.server_streaming(#path, codec, request).await
        }
    }
}

fn generate_bidirectional_streaming(
    crate_name: &TokenStream,
    name: &Ident,
    path: &str,
    input_type: &Type,
    output_type: &Type,
) -> TokenStream {
    quote! {
        pub async fn #name(&self, request: #crate_name::Request<#crate_name::Streaming<#input_type>>) -> ::std::result::Result<#crate_name::Response<#crate_name::Streaming<#output_type>>, #crate_name::Status> {
            let codec = <#crate_name::codec::ProstCodec<_, _> as ::std::default::Default>::default();
            self.cli.bidirectional_streaming(#path, codec, request).await
        }
    }
}
