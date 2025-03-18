mod tools;
mod utils;

use darling::FromMeta;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemImpl};

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

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Tools(args: TokenStream, input: TokenStream) -> TokenStream {
    let tool_args = parse_nested_meta!(tools::ToolsArgs, args);
    let item_impl = parse_macro_input!(input as ItemImpl);
    match tools::generate(tool_args, item_impl) {
        Ok(stream) => stream.into(),
        Err(err) => err.write_errors().into(),
    }
}
