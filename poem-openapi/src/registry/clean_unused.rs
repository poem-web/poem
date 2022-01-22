use std::collections::BTreeSet;

use crate::registry::{Document, MetaMediaType, MetaOperation, MetaSchemaRef};

type UsedTypes = BTreeSet<&'static str>;

impl<'a> Document<'a> {
    fn traverse_schema(&self, used_types: &mut UsedTypes, schema_ref: &MetaSchemaRef) {
        let schema = match schema_ref {
            MetaSchemaRef::Reference(name) => {
                used_types.insert(*name);
                self.registry
                    .schemas
                    .get(name)
                    .unwrap_or_else(|| panic!("Schema `{}` does not registered", name))
            }
            MetaSchemaRef::Inline(schema) => schema,
        };

        for (_, schema_ref) in &schema.properties {
            self.traverse_schema(used_types, schema_ref);
        }

        for schema_ref in &schema.items {
            self.traverse_schema(used_types, schema_ref);
        }

        if let Some(schema_ref) = &schema.additional_properties {
            self.traverse_schema(used_types, schema_ref);
        }

        for schema_ref in &schema.any_of {
            self.traverse_schema(used_types, schema_ref);
        }

        for schema_ref in &schema.one_of {
            self.traverse_schema(used_types, schema_ref);
        }

        for schema_ref in &schema.all_of {
            self.traverse_schema(used_types, schema_ref);
        }
    }

    fn traverse_media_types(&self, used_types: &mut UsedTypes, meta_types: &[MetaMediaType]) {
        for meta_type in meta_types {
            self.traverse_schema(used_types, &meta_type.schema);
        }
    }

    fn traverse_operation(&self, used_types: &mut UsedTypes, operation: &MetaOperation) {
        for param in &operation.params {
            self.traverse_schema(used_types, &param.schema);
        }

        if let Some(request) = &operation.request {
            self.traverse_media_types(used_types, &request.content);
        }

        for response in &operation.responses.responses {
            self.traverse_media_types(used_types, &response.content);
        }
    }

    pub(crate) fn remove_unused_schemas(&mut self) {
        let mut used_types = UsedTypes::new();

        for api in self.apis.iter() {
            for path in api.paths.iter() {
                for operation in &path.operations {
                    self.traverse_operation(&mut used_types, operation);
                }
            }
        }

        for api in self.webhooks.iter() {
            self.traverse_operation(&mut used_types, &api.operation);
        }

        let all_schemas = self
            .registry
            .schemas
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        for name in all_schemas.difference(&used_types).collect::<Vec<_>>() {
            self.registry.schemas.remove(name);
        }
    }
}
