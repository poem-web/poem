use std::{borrow::Cow, collections::BTreeMap};

use indexmap::IndexMap;
use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::registry::{
    MetaApi, MetaExternalDocument, MetaInfo, MetaPath, MetaResponses, MetaSchema, MetaSchemaRef,
    MetaSecurityScheme, MetaServer, MetaWebhook, Registry,
};

const OPENAPI_VERSION: &str = "3.0.0";

impl Serialize for MetaSchemaRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MetaSchemaRef::Inline(schema) => schema.serialize(serializer),
            MetaSchemaRef::Reference(name) => {
                let mut s = serializer.serialize_map(None)?;
                s.serialize_entry("$ref", &format!("#/components/schemas/{name}"))?;
                s.end()
            }
        }
    }
}

struct PathMap<'a>(&'a [MetaApi], Option<&'a str>);

impl Serialize for PathMap<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Combined APIs (e.g. `(QueryApi, CommandApi)`) can each have operations on the same
        // path. They are routed together, so they are written as one path item instead of
        // repeating the key
        let mut paths: IndexMap<Cow<'_, str>, Vec<&MetaPath>> = IndexMap::new();
        for api in self.0 {
            for path in &api.paths {
                let key = match self.1 {
                    Some(p) => Cow::Owned(format!("{}{}", p, path.path)),
                    None => Cow::Borrowed(path.path.as_str()),
                };
                paths.entry(key).or_default().push(path);
            }
        }

        let mut s = serializer.serialize_map(Some(paths.len()))?;
        for (key, paths) in &paths {
            s.serialize_entry(key, &PathItem(paths))?;
        }
        s.end()
    }
}

impl Serialize for MetaPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PathItem(&[self]).serialize(serializer)
    }
}

struct PathItem<'a>(&'a [&'a MetaPath]);

impl Serialize for PathItem<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_map(None)?;

        for path in self.0 {
            for operation in &path.operations {
                s.serialize_entry(&operation.method.to_string().to_lowercase(), operation)?;
            }
        }

        s.end()
    }
}

impl Serialize for MetaResponses {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_map(None)?;
        for resp in &self.responses {
            match resp.status {
                Some(status) => s.serialize_entry(&format!("{status}"), resp)?,
                None => match &resp.status_range {
                    Some(status_range) => s.serialize_entry(status_range, resp)?,
                    None => s.serialize_entry("default", resp)?,
                },
            }
        }
        s.end()
    }
}

struct WebhookMap<'a>(&'a [MetaWebhook]);

impl Serialize for WebhookMap<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_map(Some(self.0.len()))?;
        for webhook in self.0 {
            let mut inner_map = BTreeMap::new();
            inner_map.insert(
                webhook.operation.method.to_string().to_lowercase(),
                &webhook.operation,
            );
            s.serialize_entry(&webhook.name, &inner_map)?;
        }
        s.end()
    }
}

pub(crate) struct Document<'a> {
    pub(crate) info: &'a MetaInfo,
    pub(crate) servers: &'a [MetaServer],
    pub(crate) apis: Vec<MetaApi>,
    pub(crate) webhooks: Vec<MetaWebhook>,
    pub(crate) registry: Registry,
    pub(crate) external_document: Option<&'a MetaExternalDocument>,
    pub(crate) url_prefix: Option<&'a str>,
}

impl Serialize for Document<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Components<'a> {
            schemas: &'a BTreeMap<String, MetaSchema>,
            #[serde(skip_serializing_if = "BTreeMap::is_empty")]
            security_schemes: &'a BTreeMap<&'static str, MetaSecurityScheme>,
        }

        let mut s = serializer.serialize_map(None)?;

        s.serialize_entry("openapi", OPENAPI_VERSION)?;
        s.serialize_entry("info", &self.info)?;
        s.serialize_entry("servers", self.servers)?;
        s.serialize_entry("tags", &self.registry.tags)?;
        if !self.webhooks.is_empty() {
            s.serialize_entry("webhooks", &WebhookMap(&self.webhooks))?;
        }
        s.serialize_entry("paths", &PathMap(&self.apis, self.url_prefix))?;
        s.serialize_entry(
            "components",
            &Components {
                schemas: &self.registry.schemas,
                security_schemes: &self.registry.security_schemes,
            },
        )?;

        if let Some(external_document) = self.external_document {
            s.serialize_entry("externalDocs", &external_document)?;
        }

        s.end()
    }
}
