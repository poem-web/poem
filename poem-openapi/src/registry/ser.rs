use std::collections::BTreeMap;

use serde::{ser::SerializeMap, Serialize, Serializer};

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
                s.serialize_entry("$ref", &format!("#/components/schemas/{}", name))?;
                s.end()
            }
        }
    }
}

struct PathMap<'a>(&'a [MetaApi]);

impl<'a> Serialize for PathMap<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_map(Some(self.0.len()))?;
        for api in self.0 {
            for path in &api.paths {
                s.serialize_entry(path.path, path)?;
            }
        }
        s.end()
    }
}

impl Serialize for MetaPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_map(None)?;

        for operation in &self.operations {
            s.serialize_entry(&operation.method.to_string().to_lowercase(), operation)?;
        }

        s.end()
    }
}

impl Serialize for MetaResponses {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_map(None)?;
        for resp in &self.responses {
            match resp.status {
                Some(status) => s.serialize_entry(&format!("{}", status), resp)?,
                None => s.serialize_entry("default", resp)?,
            }
        }
        s.end()
    }
}

struct WebhookMap<'a>(&'a [MetaWebhook]);

impl<'a> Serialize for WebhookMap<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_map(Some(self.0.len()))?;
        for webhook in self.0 {
            s.serialize_entry(&webhook.name, &webhook.operation)?;
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
}

impl<'a> Serialize for Document<'a> {
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
        s.serialize_entry("paths", &PathMap(&self.apis))?;
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
