use darling::FromMeta;
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Lit, Meta, NestedMeta, Path};

#[derive(Debug, Copy, Clone, FromMeta)]
pub(crate) enum RenameRule {
    #[darling(rename = "lowercase")]
    Lower,
    #[darling(rename = "UPPERCASE")]
    Upper,
    #[darling(rename = "PascalCase")]
    Pascal,
    #[darling(rename = "camelCase")]
    Camel,
    #[darling(rename = "snake_case")]
    Snake,
    #[darling(rename = "SCREAMING_SNAKE_CASE")]
    ScreamingSnake,
}

impl RenameRule {
    pub(crate) fn rename(self, name: impl AsRef<str>) -> String {
        match self {
            Self::Lower => name.as_ref().to_lowercase(),
            Self::Upper => name.as_ref().to_uppercase(),
            Self::Pascal => name.as_ref().to_pascal_case(),
            Self::Camel => name.as_ref().to_camel_case(),
            Self::Snake => name.as_ref().to_snake_case(),
            Self::ScreamingSnake => name.as_ref().to_screaming_snake_case(),
        }
    }
}

pub(crate) trait RenameRuleExt {
    fn rename(&self, name: impl AsRef<str>) -> String;
}

impl RenameRuleExt for Option<RenameRule> {
    fn rename(&self, name: impl AsRef<str>) -> String {
        match self {
            Some(rule) => rule.rename(name),
            None => name.as_ref().to_string(),
        }
    }
}

#[derive(FromMeta)]
pub(crate) struct ConcreteType {
    pub(crate) name: String,
    pub(crate) params: PathList,
    #[darling(default)]
    pub(crate) example: Option<Path>,
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
