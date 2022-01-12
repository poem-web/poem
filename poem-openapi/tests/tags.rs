use std::collections::BTreeSet;

use poem_openapi::{
    registry::{MetaExternalDocument, MetaTag, Registry},
    Tags,
};

#[tokio::test]
async fn rename_all() {
    #[derive(Tags)]
    #[oai(rename_all = "camelCase")]
    enum MyTags {
        UserOperations,
        PetOperations,
    }

    assert_eq!(MyTags::UserOperations.name(), "userOperations");
    assert_eq!(MyTags::PetOperations.name(), "petOperations");
}

#[tokio::test]
async fn default_name() {
    #[derive(Tags)]
    enum MyTags {
        UserOperations,
        PetOperations,
    }

    assert_eq!(MyTags::UserOperations.name(), "UserOperations");
    assert_eq!(MyTags::PetOperations.name(), "PetOperations");
}

#[tokio::test]
async fn rename_item() {
    #[derive(Tags)]
    enum MyTags {
        #[oai(rename = "userOperations")]
        UserOperations,
        PetOperations,
    }

    assert_eq!(MyTags::UserOperations.name(), "userOperations");
    assert_eq!(MyTags::PetOperations.name(), "PetOperations");
}

#[tokio::test]
async fn meta() {
    #[derive(Tags)]
    #[allow(dead_code)]
    enum MyTags {
        /// User operations
        UserOperations,
        /// Pet operations
        PetOperations,
    }

    let mut registry = Registry::new();
    MyTags::UserOperations.register(&mut registry);
    assert_eq!(
        registry.tags,
        vec![
            MetaTag {
                name: "UserOperations",
                description: Some("User operations"),
                external_docs: None
            },
            MetaTag {
                name: "PetOperations",
                description: Some("Pet operations"),
                external_docs: None
            }
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
    );
}

#[tokio::test]
async fn external_docs() {
    #[derive(Tags)]
    #[allow(dead_code)]
    enum MyTags {
        #[oai(
            external_docs = "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
        )]
        UserOperations,
    }

    let mut registry = Registry::new();
    MyTags::UserOperations.register(&mut registry);
    assert_eq!(
        registry.tags.into_iter().next().unwrap(),
        MetaTag {
            name: "UserOperations",
            description: None,
            external_docs: Some(MetaExternalDocument {
                url: "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
                    .to_string(),
                description: None
            })
        }
    );
}
