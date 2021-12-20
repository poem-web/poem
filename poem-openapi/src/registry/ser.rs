use std::collections::{BTreeMap, HashMap};

use serde::{
    ser::{SerializeMap, SerializeStruct},
    Serialize, Serializer,
};

use crate::registry::{
    MetaApi, MetaExternalDocument, MetaInfo, MetaPath, MetaResponses, MetaSchema, MetaSchemaRef,
    MetaSecurityScheme, MetaServer, MetaWebhook, Registry,
};

const OPENAPI_VERSION: &str = "3.0.0";

impl<'a> Serialize for MetaSchemaRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MetaSchemaRef::Inline(schema) => schema.serialize(serializer),
            MetaSchemaRef::Reference(name) => {
                let mut s = serializer.serialize_struct("MetaSchemaRef", 1)?;
                s.serialize_field("$ref", &format!("#/components/schemas/{}", name))?;
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
    pub(crate) apis: &'a [MetaApi],
    pub(crate) webhooks: &'a [MetaWebhook],
    pub(crate) registry: &'a Registry,
    pub(crate) external_document: Option<&'a MetaExternalDocument>,
}

impl<'a> Serialize for Document<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Components<'a> {
            schemas: &'a HashMap<&'static str, MetaSchema>,
            #[serde(rename = "securitySchemes")]
            #[serde(skip_serializing_if = "BTreeMap::is_empty")]
            security_schemes: &'a BTreeMap<&'static str, MetaSecurityScheme>,
        }

        let mut s = serializer.serialize_struct("OpenAPI", 6)?;

        s.serialize_field("openapi", OPENAPI_VERSION)?;
        s.serialize_field("info", &self.info)?;
        s.serialize_field("servers", self.servers)?;
        s.serialize_field("tags", &self.registry.tags)?;
        if !self.webhooks.is_empty() {
            s.serialize_field("webhooks", &WebhookMap(self.webhooks))?;
        }
        s.serialize_field("paths", &PathMap(self.apis))?;
        s.serialize_field(
            "components",
            &Components {
                schemas: &self.registry.schemas,
                security_schemes: &self.registry.security_schemes,
            },
        )?;
        s.serialize_field("externalDocs", &self.external_document)?;

        s.end()
    }
}
