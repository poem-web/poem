//! Macros for poem-openapi

#![doc(html_favicon_url = "https://raw.githubusercontent.com/poem-web/poem/master/favicon.ico")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/poem-web/poem/master/logo.png")]
#![forbid(unsafe_code)]
#![deny(unreachable_pub)]

#[macro_use]
mod validators;

mod api;
mod common_args;
mod r#enum;
mod error;
mod multipart;
mod newtype;
mod oauth_scopes;
mod object;
mod request;
mod response;
mod response_content;
mod security_scheme;
mod tags;
mod union;
mod utils;
mod webhook;

mod parameter_style;

use darling::FromMeta;
use proc_macro::TokenStream;
use syn::{DeriveInput, ItemImpl, ItemTrait, parse_macro_input};

macro_rules! parse_nested_meta {
    ($ty:ty, $args:expr) => {{
        let meta = match darling::ast::NestedMeta::parse_meta_list(proc_macro2::TokenStream::from(
            $args,
        )) {
            Ok(v) => v,
            Err(e) => {
                return TokenStream::from(darling::Error::from(e).write_errors());
            }
        };

        match <$ty>::from_list(&meta) {
            Ok(object_args) => object_args,
            Err(err) => return TokenStream::from(err.write_errors()),
        }
    }};
}

#[proc_macro_derive(Object, attributes(oai))]
pub fn derive_object(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match object::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(Enum, attributes(oai))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match r#enum::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(Union, attributes(oai))]
pub fn derive_union(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match union::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(ApiResponse, attributes(oai))]
pub fn derive_response(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match response::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(ApiRequest, attributes(oai))]
pub fn derive_request(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match request::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(ResponseContent, attributes(oai))]
pub fn derive_response_content(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match response_content::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn OpenApi(args: TokenStream, input: TokenStream) -> TokenStream {
    let api_args = parse_nested_meta!(api::APIArgs, args);
    let item_impl = parse_macro_input!(input as ItemImpl);
    match api::generate(api_args, item_impl) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(Multipart, attributes(oai))]
pub fn derive_multipart(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match multipart::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(Tags, attributes(oai))]
pub fn derive_tags(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match tags::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(OAuthScopes, attributes(oai))]
pub fn derive_oauth_scopes(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match oauth_scopes::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(SecurityScheme, attributes(oai))]
pub fn derive_security_scheme(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match security_scheme::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Webhook(args: TokenStream, input: TokenStream) -> TokenStream {
    let webhook_args = parse_nested_meta!(webhook::WebhookArgs, args);
    let item_trait = parse_macro_input!(input as ItemTrait);
    match webhook::generate(webhook_args, item_trait) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(NewType, attributes(oai))]
pub fn derive_new_type(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DeriveInput);
    match newtype::generate(args) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}
