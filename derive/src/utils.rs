use proc_macro2::{Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::Ident;

pub(crate) fn get_crate_name() -> TokenStream {
    let name = match crate_name("poem") {
        Ok(FoundCrate::Name(name)) => name,
        Ok(FoundCrate::Itself) | Err(_) => "poem".to_string(),
    };
    let name = Ident::new(&name, Span::call_site());
    quote!(#name)
}
