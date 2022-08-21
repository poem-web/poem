use darling::{util::SpannedValue, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Lit, Meta, NestedMeta, Path};

#[derive(Debug, Copy, Clone, FromMeta)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum RenameRule {
    #[darling(rename = "lowercase")]
    LowerCase,
    #[darling(rename = "UPPERCASE")]
    UpperCase,
    #[darling(rename = "PascalCase")]
    PascalCase,
    #[darling(rename = "camelCase")]
    CamelCase,
    #[darling(rename = "snake_case")]
    SnakeCase,
    #[darling(rename = "SCREAMING_SNAKE_CASE")]
    ScreamingSnakeCase,
    #[darling(rename = "kebab-case")]
    KebabCase,
    #[darling(rename = "SCREAMING-KEBAB-CASE")]
    ScreamingKebabCase,
}

impl RenameRule {
    fn rename_variant(self, variant: String) -> String {
        use RenameRule::*;

        match self {
            PascalCase => variant,
            LowerCase => variant.to_ascii_lowercase(),
            UpperCase => variant.to_ascii_uppercase(),
            CamelCase => variant[..1].to_ascii_lowercase() + &variant[1..],
            SnakeCase => {
                let mut snake = String::new();
                for (i, ch) in variant.char_indices() {
                    if i > 0 && ch.is_uppercase() {
                        snake.push('_');
                    }
                    snake.push(ch.to_ascii_lowercase());
                }
                snake
            }
            ScreamingSnakeCase => SnakeCase.rename_variant(variant).to_ascii_uppercase(),
            KebabCase => SnakeCase.rename_variant(variant).replace('_', "-"),
            ScreamingKebabCase => ScreamingSnakeCase.rename_variant(variant).replace('_', "-"),
        }
    }

    fn rename_field(self, field: String) -> String {
        use RenameRule::*;
        match self {
            LowerCase | SnakeCase => field,
            UpperCase => field.to_ascii_uppercase(),
            PascalCase => {
                let mut pascal = String::new();
                let mut capitalize = true;
                for ch in field.chars() {
                    if ch == '_' {
                        capitalize = true;
                    } else if capitalize {
                        pascal.push(ch.to_ascii_uppercase());
                        capitalize = false;
                    } else {
                        pascal.push(ch);
                    }
                }
                pascal
            }
            CamelCase => {
                let pascal = PascalCase.rename_field(field);
                pascal[..1].to_ascii_lowercase() + &pascal[1..]
            }
            ScreamingSnakeCase => field.to_ascii_uppercase(),
            KebabCase => field.replace('_', "-"),
            ScreamingKebabCase => ScreamingSnakeCase.rename_field(field).replace('_', "-"),
        }
    }
}

pub(crate) fn apply_rename_rule_field(rule: Option<RenameRule>, field: String) -> String {
    match rule {
        Some(rule) => rule.rename_field(field),
        None => field,
    }
}

pub(crate) fn apply_rename_rule_variant(rule: Option<RenameRule>, variant: String) -> String {
    match rule {
        Some(rule) => rule.rename_variant(variant),
        None => variant,
    }
}

pub(crate) struct PathList(pub(crate) Vec<Path>);

impl FromMeta for PathList {
    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        let mut res = Vec::new();
        for item in items {
            if let NestedMeta::Meta(Meta::Path(p)) = item {
                res.push(p.clone());
            } else {
                return Err(darling::Error::custom("Invalid path list"));
            }
        }
        Ok(PathList(res))
    }
}

#[derive(Debug, Copy, Clone, FromMeta, Eq, PartialEq, Hash)]
#[darling(rename_all = "lowercase")]
pub(crate) enum APIMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Connect,
    Patch,
    Trace,
}

impl APIMethod {
    pub(crate) fn to_http_method(self) -> TokenStream {
        match self {
            APIMethod::Get => quote!(GET),
            APIMethod::Post => quote!(POST),
            APIMethod::Put => quote!(PUT),
            APIMethod::Delete => quote!(DELETE),
            APIMethod::Head => quote!(HEAD),
            APIMethod::Options => quote!(OPTIONS),
            APIMethod::Connect => quote!(CONNECT),
            APIMethod::Patch => quote!(PATCH),
            APIMethod::Trace => quote!(TRACE),
        }
    }
}

#[derive(Debug, Copy, Clone, FromMeta, Eq, PartialEq)]
pub(crate) enum ParamIn {
    #[darling(rename = "path")]
    Path,
    #[darling(rename = "query")]
    Query,
    #[darling(rename = "header")]
    Header,
    #[darling(rename = "cookie")]
    Cookie,
}

#[derive(Debug)]
pub(crate) enum DefaultValue {
    Default,
    Function(Path),
}

impl FromMeta for DefaultValue {
    fn from_word() -> darling::Result<Self> {
        Ok(DefaultValue::Default)
    }

    fn from_value(value: &Lit) -> darling::Result<Self> {
        match value {
            Lit::Str(str) => Ok(DefaultValue::Function(syn::parse_str(&str.value())?)),
            _ => Err(darling::Error::unexpected_lit_type(value).with_span(value)),
        }
    }
}

#[derive(FromMeta, Clone)]
pub(crate) struct MaximumValidator {
    pub(crate) value: f64,
    #[darling(default)]
    pub(crate) exclusive: bool,
}

#[derive(FromMeta, Clone)]
pub(crate) struct MinimumValidator {
    pub(crate) value: f64,
    #[darling(default)]
    pub(crate) exclusive: bool,
}

pub(crate) struct ExternalDocument {
    url: String,
}

impl FromMeta for ExternalDocument {
    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(ExternalDocument {
            url: value.to_string(),
        })
    }
}

impl ExternalDocument {
    pub(crate) fn to_token_stream(&self, crate_name: &TokenStream) -> TokenStream {
        let url = &self.url;
        quote! {
            #crate_name::registry::MetaExternalDocument {
                url: #url.to_string(),
                description: ::std::option::Option::None,
            }
        }
    }
}

#[derive(FromMeta)]
pub(crate) struct ExtraHeader {
    pub(crate) name: String,
    #[darling(rename = "type")]
    pub(crate) ty: SpannedValue<String>,
    #[darling(default)]
    pub(crate) description: Option<String>,
    #[darling(default)]
    pub(crate) deprecated: bool,
}

#[derive(FromMeta)]
pub(crate) struct CodeSample {
    pub(crate) lang: String,
    pub(crate) label: Option<String>,
    pub(crate) source: syn::Expr,
}
