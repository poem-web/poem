use darling::FromMeta;
use inflector::Inflector;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{Meta, NestedMeta, Path};

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

#[derive(Debug, Copy, Clone)]
pub(crate) enum RenameTarget {
    Type,
    EnumItem,
    Field,
    Tag,
    SecurityScheme,
}

impl RenameTarget {
    pub(crate) fn rule(self) -> RenameRule {
        match self {
            RenameTarget::Type => RenameRule::Pascal,
            RenameTarget::EnumItem => RenameRule::Pascal,
            RenameTarget::Field => RenameRule::Camel,
            RenameTarget::Tag => RenameRule::Snake,
            RenameTarget::SecurityScheme => RenameRule::Snake,
        }
    }

    pub(crate) fn rename(self, name: impl AsRef<str>) -> String {
        self.rule().rename(name)
    }
}

pub(crate) trait RenameRuleExt {
    fn rename(&self, name: impl AsRef<str>, target: RenameTarget) -> String;
}

impl RenameRuleExt for Option<RenameRule> {
    fn rename(&self, name: impl AsRef<str>, target: RenameTarget) -> String {
        self.unwrap_or(target.rule()).rename(name)
    }
}

#[derive(FromMeta)]
pub(crate) struct ConcreteType {
    pub(crate) name: String,
    pub(crate) params: PathList,
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

#[derive(Debug, Copy, Clone, FromMeta)]
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
    Function(Ident),
}

impl FromMeta for DefaultValue {
    fn from_word() -> darling::Result<Self> {
        Ok(DefaultValue::Default)
    }

    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(DefaultValue::Function(Ident::new(value, Span::call_site())))
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
