use darling::{
    ast::{Data, Fields},
    util::Ignored,
    FromDeriveInput, FromVariant,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ext::IdentExt, Attribute, DeriveInput, Error};

use crate::{
    common_args::{apply_rename_rule_variant, RenameRule},
    error::GeneratorResult,
    utils::{get_crate_name, get_description, optional_literal},
};

#[derive(FromVariant)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ScopeItem {
    ident: Ident,
    fields: Fields<Ignored>,
    attrs: Vec<Attribute>,

    #[darling(default)]
    rename: Option<String>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct ScopeArgs {
    ident: Ident,
    data: Data<ScopeItem, Ignored>,

    #[darling(default)]
    internal: bool,
    #[darling(default)]
    rename_all: Option<RenameRule>,
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: ScopeArgs = ScopeArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let ident = &args.ident;

    let e = match &args.data {
        Data::Enum(e) => e,
        _ => {
            return Err(
                Error::new_spanned(ident, "OAuthScopes can only be applied to an enum.").into(),
            )
        }
    };

    let mut meta_items = Vec::new();
    let mut to_names = Vec::new();

    for variant in e {
        if !variant.fields.is_empty() {
            return Err(Error::new_spanned(
                &variant.ident,
                format!(
                    "Invalid enum variant {}.\nOpenAPI oauth scopes may only contain unit variants.",
                    variant.ident
                ),
            )
            .into());
        }

        let item_ident = &variant.ident;
        let oai_item_name = variant.rename.clone().unwrap_or_else(|| {
            apply_rename_rule_variant(args.rename_all, item_ident.unraw().to_string())
        });
        let description = get_description(&variant.attrs)?;
        let description = optional_literal(&description);

        meta_items.push(quote!(#crate_name::registry::MetaOAuthScope {
            name: #oai_item_name,
            description: #description,
        }));
        to_names.push(quote!(Self::#item_ident => #oai_item_name));
    }

    let expanded = quote! {
        impl #crate_name::OAuthScopes for #ident {
            fn meta() -> ::std::vec::Vec<#crate_name::registry::MetaOAuthScope> {
                ::std::vec![#(#meta_items),*]
            }

            fn name(&self) -> &'static str {
                match self {
                #(#to_names),*
                }
            }
        }
    };

    Ok(expanded)
}
