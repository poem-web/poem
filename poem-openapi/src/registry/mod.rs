mod clean_unused;
mod ser;

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap},
    hash::{Hash, Hasher},
};

use poem::http::Method;
pub(crate) use ser::Document;
use serde::{Serialize, Serializer, ser::SerializeMap};
use serde_json::Value;

use crate::{ParameterStyle, types::Type};

#[allow(clippy::trivially_copy_pass_by_ref)]
#[inline]
const fn is_false(value: &bool) -> bool {
    !*value
}

/// OpenAPI 3.2 Discriminator Object
///
/// New in 3.2.0: `default_mapping` field for fallback schema.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaDiscriminatorObject {
    pub property_name: &'static str,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_mapping"
    )]
    pub mapping: Vec<(String, String)>,
    /// Default schema reference when discriminator value is missing or unrecognized (OpenAPI 3.2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_mapping: Option<&'static str>,
}

fn serialize_mapping<S: Serializer>(
    mapping: &[(String, String)],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_map(None)?;
    for (name, ref_name) in mapping {
        s.serialize_entry(name, ref_name)?;
    }
    s.end()
}

/// OpenAPI 3.2 Schema Object
///
/// Custom serialization is used to handle OpenAPI 3.1+ changes:
/// - `nullable` is represented as a type array (e.g., `["string", "null"]`)
/// - `exclusiveMinimum`/`exclusiveMaximum` are numeric values, not booleans
#[derive(Debug, Clone, PartialEq)]
pub struct MetaSchema {
    pub rust_typename: Option<&'static str>,

    pub ty: &'static str,
    pub format: Option<&'static str>,
    pub title: Option<String>,
    pub description: Option<&'static str>,
    pub external_docs: Option<MetaExternalDocument>,
    pub default: Option<Value>,
    pub required: Vec<&'static str>,
    pub properties: Vec<(&'static str, MetaSchemaRef)>,
    pub items: Option<Box<MetaSchemaRef>>,
    pub additional_properties: Option<Box<MetaSchemaRef>>,
    pub enum_items: Vec<Value>,
    pub deprecated: bool,
    pub nullable: bool,
    pub any_of: Vec<MetaSchemaRef>,
    pub one_of: Vec<MetaSchemaRef>,
    pub all_of: Vec<MetaSchemaRef>,
    pub discriminator: Option<MetaDiscriminatorObject>,
    pub read_only: bool,
    pub write_only: bool,
    pub example: Option<Value>,

    pub multiple_of: Option<f64>,
    pub maximum: Option<f64>,
    pub exclusive_maximum: Option<bool>,
    pub minimum: Option<f64>,
    pub exclusive_minimum: Option<bool>,
    pub max_length: Option<usize>,
    pub min_length: Option<usize>,
    pub pattern: Option<String>,
    pub max_items: Option<usize>,
    pub min_items: Option<usize>,
    pub unique_items: Option<bool>,
    pub max_properties: Option<usize>,
    pub min_properties: Option<usize>,
}

impl Serialize for MetaSchema {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;

        // Handle type field - in OpenAPI 3.1+, nullable is expressed as type array
        if !self.ty.is_empty() {
            if self.nullable {
                map.serialize_entry("type", &[self.ty, "null"])?;
            } else {
                map.serialize_entry("type", self.ty)?;
            }
        }

        if let Some(format) = &self.format {
            map.serialize_entry("format", format)?;
        }
        if let Some(title) = &self.title {
            map.serialize_entry("title", title)?;
        }
        if let Some(description) = &self.description {
            map.serialize_entry("description", description)?;
        }
        if let Some(external_docs) = &self.external_docs {
            map.serialize_entry("externalDocs", external_docs)?;
        }
        if let Some(default) = &self.default {
            map.serialize_entry("default", default)?;
        }
        if !self.required.is_empty() {
            map.serialize_entry("required", &self.required)?;
        }
        if !self.properties.is_empty() {
            map.serialize_entry("properties", &PropertiesSerializer(&self.properties))?;
        }
        if let Some(items) = &self.items {
            map.serialize_entry("items", items)?;
        }
        if let Some(additional_properties) = &self.additional_properties {
            map.serialize_entry("additionalProperties", additional_properties)?;
        }
        if !self.enum_items.is_empty() {
            map.serialize_entry("enum", &self.enum_items)?;
        }
        if self.deprecated {
            map.serialize_entry("deprecated", &true)?;
        }
        if !self.any_of.is_empty() {
            map.serialize_entry("anyOf", &self.any_of)?;
        }
        if !self.one_of.is_empty() {
            map.serialize_entry("oneOf", &self.one_of)?;
        }
        if !self.all_of.is_empty() {
            map.serialize_entry("allOf", &self.all_of)?;
        }
        if let Some(discriminator) = &self.discriminator {
            map.serialize_entry("discriminator", discriminator)?;
        }
        if self.read_only {
            map.serialize_entry("readOnly", &true)?;
        }
        if self.write_only {
            map.serialize_entry("writeOnly", &true)?;
        }
        if let Some(example) = &self.example {
            map.serialize_entry("example", example)?;
        }
        if let Some(multiple_of) = &self.multiple_of {
            map.serialize_entry("multipleOf", multiple_of)?;
        }

        // Handle exclusive minimum/maximum - in OpenAPI 3.1+, these are numeric values
        // In 3.0: {"minimum": 0, "exclusiveMinimum": true} means > 0
        // In 3.1+: {"exclusiveMinimum": 0} means > 0
        match (self.minimum, self.exclusive_minimum) {
            (Some(min), Some(true)) => {
                map.serialize_entry("exclusiveMinimum", &min)?;
            }
            (Some(min), _) => {
                map.serialize_entry("minimum", &min)?;
            }
            _ => {}
        }

        match (self.maximum, self.exclusive_maximum) {
            (Some(max), Some(true)) => {
                map.serialize_entry("exclusiveMaximum", &max)?;
            }
            (Some(max), _) => {
                map.serialize_entry("maximum", &max)?;
            }
            _ => {}
        }

        if let Some(max_length) = &self.max_length {
            map.serialize_entry("maxLength", max_length)?;
        }
        if let Some(min_length) = &self.min_length {
            map.serialize_entry("minLength", min_length)?;
        }
        if let Some(pattern) = &self.pattern {
            map.serialize_entry("pattern", pattern)?;
        }
        if let Some(max_items) = &self.max_items {
            map.serialize_entry("maxItems", max_items)?;
        }
        if let Some(min_items) = &self.min_items {
            map.serialize_entry("minItems", min_items)?;
        }
        if let Some(unique_items) = &self.unique_items {
            map.serialize_entry("uniqueItems", unique_items)?;
        }
        if let Some(max_properties) = &self.max_properties {
            map.serialize_entry("maxProperties", max_properties)?;
        }
        if let Some(min_properties) = &self.min_properties {
            map.serialize_entry("minProperties", min_properties)?;
        }

        map.end()
    }
}

/// Helper struct to serialize properties as a map
struct PropertiesSerializer<'a>(&'a [(&'static str, MetaSchemaRef)]);

impl Serialize for PropertiesSerializer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, schema) in self.0 {
            map.serialize_entry(name, schema)?;
        }
        map.end()
    }
}

impl MetaSchema {
    pub const ANY: Self = MetaSchema {
        rust_typename: None,
        ty: "",
        format: None,
        title: None,
        description: None,
        external_docs: None,
        default: None,
        required: vec![],
        properties: vec![],
        items: None,
        additional_properties: None,
        enum_items: vec![],
        deprecated: false,
        any_of: vec![],
        one_of: vec![],
        all_of: vec![],
        discriminator: None,
        read_only: false,
        write_only: false,
        nullable: false,
        example: None,
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
        max_properties: None,
        min_properties: None,
    };

    pub fn new(ty: &'static str) -> Self {
        Self { ty, ..Self::ANY }
    }

    pub fn new_with_format(ty: &'static str, format: &'static str) -> Self {
        MetaSchema {
            ty,
            format: Some(format),
            ..Self::ANY
        }
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::ANY
    }

    #[must_use]
    pub fn merge(
        mut self,
        MetaSchema {
            default,
            read_only,
            write_only,
            deprecated,
            nullable,
            title,
            description,
            external_docs,
            items,
            additional_properties,
            example,
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
            max_properties,
            min_properties,
            ..
        }: MetaSchema,
    ) -> Self {
        self.read_only |= read_only;
        self.write_only |= write_only;
        self.nullable |= nullable;
        self.deprecated |= deprecated;

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
            external_docs,
            example,
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
            max_properties,
            min_properties
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
                            any_of: vec![*self_items, items],
                            ..MetaSchema::ANY
                        }))));
                    }
                }
            } else {
                self.items = Some(items);
            }
        }

        if let Some(additional_properties) = additional_properties {
            if let Some(self_additional_properties) = self.additional_properties {
                let additional_properties = *additional_properties;

                match additional_properties {
                    MetaSchemaRef::Inline(additional_properties) => {
                        self.additional_properties = Some(Box::new(
                            self_additional_properties.merge(*additional_properties),
                        ))
                    }
                    MetaSchemaRef::Reference(_) => {
                        self.additional_properties =
                            Some(Box::new(MetaSchemaRef::Inline(Box::new(MetaSchema {
                                any_of: vec![*self_additional_properties, additional_properties],
                                ..MetaSchema::ANY
                            }))));
                    }
                }
            } else {
                self.additional_properties = Some(additional_properties);
            }
        }

        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetaSchemaRef {
    Inline(Box<MetaSchema>),
    Reference(String),
}

impl MetaSchemaRef {
    pub fn is_array(&self) -> bool {
        matches!(self, MetaSchemaRef::Inline(schema) if schema.ty == "array")
    }

    pub fn is_object(&self) -> bool {
        matches!(self, MetaSchemaRef::Inline(schema) if schema.ty == "object")
    }

    pub fn unwrap_inline(&self) -> &MetaSchema {
        match &self {
            MetaSchemaRef::Inline(schema) => schema,
            MetaSchemaRef::Reference(_) => panic!(),
        }
    }

    pub fn unwrap_reference(&self) -> &str {
        match self {
            MetaSchemaRef::Inline(_) => panic!(),
            MetaSchemaRef::Reference(name) => name,
        }
    }

    #[must_use]
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
                            MetaSchemaRef::Inline(Box::new(other.clone())),
                        ],
                        ..other
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
    pub name: String,
    pub schema: MetaSchemaRef,
    #[serde(rename = "in")]
    pub in_type: MetaParamIn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: bool,
    pub deprecated: bool,
    pub explode: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<ParameterStyle>,
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
#[serde(rename_all = "camelCase")]
pub struct MetaHeader {
    #[serde(skip)]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub required: bool,
    pub deprecated: bool,
    pub schema: MetaSchemaRef,
}

/// OpenAPI 3.2 Response Object
///
/// New in 3.2.0: `summary` field and `description` is now optional.
#[derive(Debug, PartialEq, Serialize)]
pub struct MetaResponse {
    /// Brief summary of the response (OpenAPI 3.2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<&'static str>,
    /// Description is now optional in OpenAPI 3.2.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip)]
    pub status: Option<u16>,
    #[serde(skip)]
    pub status_range: Option<String>,
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
pub struct MetaWebhook {
    pub name: &'static str,
    pub operation: MetaOperation,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaCodeSample {
    pub lang: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<&'static str>,
    pub source: &'static str,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<MetaExternalDocument>,
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
    #[serde(rename = "x-code-samples", skip_serializing_if = "Vec::is_empty")]
    pub code_samples: Vec<MetaCodeSample>,
}

#[derive(Debug, PartialEq)]
pub struct MetaPath {
    pub path: String,
    pub operations: Vec<MetaOperation>,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize, Clone)]
pub struct MetaContact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize, Clone)]
pub struct MetaLicense {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetaInfo {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_of_service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<MetaContact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<MetaLicense>,
}

/// OpenAPI 3.2 Server Object
///
/// New in 3.2.0: `name` field for server identification.
#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
pub struct MetaServer {
    pub url: String,
    /// Server name for identification (OpenAPI 3.2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub variables: BTreeMap<String, MetaServerVariable>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
pub struct MetaServerVariable {
    pub default: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,

    #[serde(rename = "enum", skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct MetaExternalDocument {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// OpenAPI 3.2 Tag Object
///
/// New in 3.2.0: `summary`, `parent`, and `kind` fields for multipurpose tags.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaTag {
    pub name: &'static str,
    /// A brief summary of the tag (OpenAPI 3.2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<MetaExternalDocument>,
    /// Parent tag name for hierarchical tag organization (OpenAPI 3.2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<&'static str>,
    /// Tag kind for categorization (e.g., "nav", "audience") (OpenAPI 3.2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
}

impl PartialEq for MetaTag {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(other.name)
    }
}

impl Eq for MetaTag {}

impl Ord for MetaTag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(other.name)
    }
}

impl PartialOrd for MetaTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for MetaTag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MetaOAuthScope {
    pub name: &'static str,
    pub description: Option<&'static str>,
}

/// OpenAPI 3.2 OAuth Flow Object
///
/// New in 3.2.0: `device_authorization_url` for Device Authorization Grant flow.
#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaOAuthFlow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_url: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<&'static str>,
    /// Device authorization URL for Device Authorization Grant (OpenAPI 3.2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_authorization_url: Option<&'static str>,
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

/// OpenAPI 3.2 OAuth Flows Object
///
/// New in 3.2.0: `device_authorization` for Device Authorization Grant flow.
#[derive(Debug, Eq, PartialEq, Serialize)]
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
    /// Device Authorization Grant flow (OpenAPI 3.2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_authorization: Option<MetaOAuthFlow>,
}

/// OpenAPI 3.2 Security Scheme Object
///
/// New in 3.2.0: `oauth2_metadata_url` for OAuth 2.0 Server Metadata.
#[derive(Debug, Serialize, Eq, PartialEq)]
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
    /// OAuth 2.0 Server Metadata URL (OpenAPI 3.2+)
    #[serde(rename = "oauth2MetadataUrl", skip_serializing_if = "Option::is_none")]
    pub oauth2_metadata_url: Option<&'static str>,
}

#[derive(Debug, PartialEq)]
pub struct MetaApi {
    pub paths: Vec<MetaPath>,
}

#[derive(Default)]
pub struct Registry {
    pub schemas: BTreeMap<String, MetaSchema>,
    pub tags: BTreeSet<MetaTag>,
    pub security_schemes: BTreeMap<&'static str, MetaSecurityScheme>,
}

impl Registry {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn create_schema<T, F>(&mut self, name: String, f: F)
    where
        F: FnOnce(&mut Registry) -> MetaSchema,
    {
        match self.schemas.get(&name) {
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
                self.schemas.insert(name.clone(), MetaSchema::new("fake"));
                let mut meta_schema = f(self);
                meta_schema.rust_typename = Some(std::any::type_name::<T>());
                *self.schemas.get_mut(&name).unwrap() = meta_schema;
            }
        }
    }

    pub fn create_fake_schema<T: Type>(&mut self) -> MetaSchema {
        match T::schema_ref() {
            MetaSchemaRef::Inline(schema) => *schema,
            MetaSchemaRef::Reference(name) => {
                T::register(self);
                self.schemas
                    .get(&name)
                    .cloned()
                    .expect("You definitely encountered a bug!")
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
