mod ser;

use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    hash::{Hash, Hasher},
};

use poem::http::Method;
pub(crate) use ser::Document;
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::Value;

#[allow(clippy::trivially_copy_pass_by_ref)]
#[inline]
const fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaDiscriminatorObject {
    pub property_name: &'static str,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_mapping"
    )]
    pub mapping: Vec<(&'static str, String)>,
}

fn serialize_mapping<S: Serializer>(
    mapping: &[(&'static str, String)],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_map(None)?;
    for (name, ref_name) in mapping {
        s.serialize_entry(name, ref_name)?;
    }
    s.end()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaSchema {
    #[serde(skip)]
    pub rust_typename: Option<&'static str>,

    #[serde(rename = "type", skip_serializing_if = "str::is_empty")]
    pub ty: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<&'static str>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_properties"
    )]
    pub properties: Vec<(&'static str, MetaSchemaRef)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<MetaSchemaRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<MetaSchemaRef>>,
    #[serde(rename = "enum", skip_serializing_if = "Vec::is_empty")]
    pub enum_items: Vec<Value>,
    #[serde(skip_serializing_if = "is_false")]
    pub deprecated: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub one_of: Vec<MetaSchemaRef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub all_of: Vec<MetaSchemaRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<MetaDiscriminatorObject>,
    #[serde(skip_serializing_if = "is_false")]
    pub read_only: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub write_only: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_items: Option<bool>,
}

fn serialize_properties<S: Serializer>(
    properties: &[(&'static str, MetaSchemaRef)],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_map(None)?;
    for item in properties {
        s.serialize_entry(item.0, &item.1)?;
    }
    s.end()
}

impl MetaSchema {
    pub const ANY: Self = MetaSchema {
        rust_typename: None,
        ty: "",
        format: None,
        title: None,
        description: None,
        default: None,
        required: vec![],
        properties: vec![],
        items: None,
        additional_properties: None,
        enum_items: vec![],
        deprecated: false,
        one_of: vec![],
        all_of: vec![],
        discriminator: None,
        read_only: false,
        write_only: false,
        multiple_of: None,
        maximum: None,
        exclusive_maximum: None,
        minimum: None,
        exclusive_minimum: None,
        max_length: None,
        min_length: None,
        pattern: None,
        max_items: None,
        min_items: None,
        unique_items: None,
    };

    pub const fn new(ty: &'static str) -> Self {
        Self {
            rust_typename: None,
            ty,
            format: None,
            title: None,
            description: None,
            default: None,
            required: vec![],
            properties: vec![],
            items: None,
            additional_properties: None,
            enum_items: vec![],
            deprecated: false,
            one_of: vec![],
            all_of: vec![],
            discriminator: None,
            read_only: false,
            write_only: false,
            multiple_of: None,
            maximum: None,
            exclusive_maximum: None,
            minimum: None,
            exclusive_minimum: None,
            max_length: None,
            min_length: None,
            pattern: None,
            max_items: None,
            min_items: None,
            unique_items: None,
        }
    }

    pub const fn new_with_format(ty: &'static str, format: &'static str) -> Self {
        MetaSchema {
            rust_typename: None,
            ty,
            format: Some(format),
            title: None,
            description: None,
            default: None,
            required: vec![],
            properties: vec![],
            items: None,
            additional_properties: None,
            enum_items: vec![],
            deprecated: false,
            one_of: vec![],
            all_of: vec![],
            discriminator: None,
            read_only: false,
            write_only: false,
            multiple_of: None,
            maximum: None,
            exclusive_maximum: None,
            minimum: None,
            exclusive_minimum: None,
            max_length: None,
            min_length: None,
            pattern: None,
            max_items: None,
            min_items: None,
            unique_items: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::ANY
    }

    pub fn merge(
        mut self,
        MetaSchema {
            default,
            read_only,
            write_only,
            title,
            description,
            multiple_of,
            maximum,
            exclusive_maximum,
            minimum,
            exclusive_minimum,
            max_length,
            min_length,
            pattern,
            max_items,
            min_items,
            unique_items,
            items,
            ..
        }: MetaSchema,
    ) -> Self {
        self.read_only |= read_only;
        self.write_only |= write_only;

        macro_rules! merge_optional {
            ($($name:ident),*) => {
                $(
                if $name.is_some() {
                    self.$name = $name;
                }
                )*
            };
        }

        merge_optional!(
            default,
            title,
            description,
            multiple_of,
            maximum,
            exclusive_maximum,
            minimum,
            exclusive_minimum,
            max_length,
            min_length,
            pattern,
            max_items,
            min_items,
            unique_items
        );

        if let Some(items) = items {
            if let Some(self_items) = self.items {
                let items = *items;

                match items {
                    MetaSchemaRef::Inline(items) => {
                        self.items = Some(Box::new(self_items.merge(*items)))
                    }
                    MetaSchemaRef::Reference(_) => {
                        self.items = Some(Box::new(MetaSchemaRef::Inline(Box::new(MetaSchema {
                            one_of: vec![*self_items, items],
                            ..MetaSchema::ANY
                        }))));
                    }
                }
            } else {
                self.items = Some(items);
            }
        }

        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetaSchemaRef {
    Inline(Box<MetaSchema>),
    Reference(&'static str),
}

impl MetaSchemaRef {
    pub fn unwrap_inline(&self) -> &MetaSchema {
        match &self {
            MetaSchemaRef::Inline(schema) => schema,
            MetaSchemaRef::Reference(_) => panic!(),
        }
    }

    pub fn unwrap_reference(&self) -> &'static str {
        match self {
            MetaSchemaRef::Inline(_) => panic!(),
            MetaSchemaRef::Reference(name) => name,
        }
    }

    pub fn merge(self, other: MetaSchema) -> Self {
        match self {
            MetaSchemaRef::Inline(schema) => MetaSchemaRef::Inline(Box::new(schema.merge(other))),
            MetaSchemaRef::Reference(name) => {
                let other = MetaSchema::ANY.merge(other);
                if other.is_empty() {
                    MetaSchemaRef::Reference(name)
                } else {
                    MetaSchemaRef::Inline(Box::new(MetaSchema {
                        all_of: vec![
                            MetaSchemaRef::Reference(name),
                            MetaSchemaRef::Inline(Box::new(other)),
                        ],
                        ..MetaSchema::ANY
                    }))
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MetaParamIn {
    Query,
    Header,
    Path,
    Cookie,
    #[serde(rename = "cookie")]
    CookiePrivate,
    #[serde(rename = "cookie")]
    CookieSigned,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct MetaOperationParam {
    pub name: &'static str,
    pub schema: MetaSchemaRef,
    #[serde(rename = "in")]
    pub in_type: MetaParamIn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    pub required: bool,
    pub deprecated: bool,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct MetaMediaType {
    #[serde(skip)]
    pub content_type: &'static str,
    pub schema: MetaSchemaRef,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct MetaRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_content"
    )]
    pub content: Vec<MetaMediaType>,
    pub required: bool,
}

fn serialize_content<S: Serializer>(
    content: &[MetaMediaType],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_map(None)?;
    for item in content {
        s.serialize_entry(item.content_type, item)?;
    }
    s.end()
}

#[derive(Debug, PartialEq)]
pub struct MetaResponses {
    pub responses: Vec<MetaResponse>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct MetaHeader {
    #[serde(skip)]
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "is_false")]
    pub required: bool,
    pub schema: MetaSchemaRef,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct MetaResponse {
    pub description: &'static str,
    #[serde(skip)]
    pub status: Option<u16>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_content"
    )]
    pub content: Vec<MetaMediaType>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_headers"
    )]
    pub headers: Vec<MetaHeader>,
}

fn serialize_headers<S: Serializer>(
    properties: &[MetaHeader],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_map(None)?;
    for header in properties {
        s.serialize_entry(&header.name, &header)?;
    }
    s.end()
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaOperation {
    #[serde(skip)]
    pub method: Method,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(rename = "parameters", skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<MetaOperationParam>,
    #[serde(rename = "requestBody", skip_serializing_if = "Option::is_none")]
    pub request: Option<MetaRequest>,
    pub responses: MetaResponses,
    #[serde(skip_serializing_if = "is_false")]
    pub deprecated: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<HashMap<&'static str, Vec<&'static str>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<&'static str>,
}

#[derive(Debug, PartialEq)]
pub struct MetaPath {
    pub path: &'static str,
    pub operations: Vec<MetaOperation>,
}

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct MetaInfo {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub version: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct MetaServer {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MetaTag {
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
}

impl PartialEq for MetaTag {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(other.name)
    }
}

impl Eq for MetaTag {}

impl PartialOrd for MetaTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(other.name)
    }
}

impl Ord for MetaTag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(other.name)
    }
}

impl Hash for MetaTag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct MetaOAuthScope {
    pub name: &'static str,
    pub description: Option<&'static str>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaOAuthFlow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_url: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<&'static str>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_oauth_flow_scopes"
    )]
    pub scopes: Vec<MetaOAuthScope>,
}

fn serialize_oauth_flow_scopes<S: Serializer>(
    properties: &[MetaOAuthScope],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_map(None)?;
    for item in properties {
        s.serialize_entry(item.name, item.description.unwrap_or_default())?;
    }
    s.end()
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaOAuthFlows {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<MetaOAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<MetaOAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_credentials: Option<MetaOAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<MetaOAuthFlow>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetaSecurityScheme {
    #[serde(rename = "type")]
    pub ty: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'static str>,
    #[serde(rename = "in", skip_serializing_if = "Option::is_none")]
    pub key_in: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flows: Option<MetaOAuthFlows>,
    #[serde(rename = "openIdConnectUrl", skip_serializing_if = "Option::is_none")]
    pub openid_connect_url: Option<&'static str>,
}

#[derive(Debug, PartialEq)]
pub struct MetaApi {
    pub paths: Vec<MetaPath>,
}

#[derive(Default)]
pub struct Registry {
    pub schemas: HashMap<&'static str, MetaSchema>,
    pub tags: HashSet<MetaTag>,
    pub security_schemes: BTreeMap<&'static str, MetaSecurityScheme>,
}

impl Registry {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn create_schema<T, F>(&mut self, name: &'static str, mut f: F)
    where
        F: FnMut(&mut Registry) -> MetaSchema,
    {
        match self.schemas.get(name) {
            Some(schema) => {
                if let Some(prev_typename) = schema.rust_typename {
                    if prev_typename != std::any::type_name::<T>() {
                        panic!(
                            "`{}` and `{}` have the same OpenAPI name `{}`",
                            prev_typename,
                            std::any::type_name::<T>(),
                            name,
                        );
                    }
                }
            }
            None => {
                // Inserting a fake type before calling the function allows recursive types to
                // exist.
                self.schemas.insert(name, MetaSchema::new("fake"));
                let mut meta_schema = f(self);
                meta_schema.rust_typename = Some(std::any::type_name::<T>());
                *self.schemas.get_mut(name).unwrap() = meta_schema;
            }
        }
    }

    pub fn create_tag(&mut self, tag: MetaTag) {
        self.tags.insert(tag);
    }

    pub fn create_security_scheme(
        &mut self,
        name: &'static str,
        security_scheme: MetaSecurityScheme,
    ) {
        if !self.security_schemes.contains_key(name) {
            self.security_schemes.insert(name, security_scheme);
        }
    }
}
