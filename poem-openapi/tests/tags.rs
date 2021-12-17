use std::collections::HashSet;

use poem_openapi::{
    registry::{MetaTag, Registry},
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
            },
            MetaTag {
                name: "PetOperations",
                description: Some("Pet operations"),
            }
        ]
        .into_iter()
        .collect::<HashSet<_>>()
    );
}
