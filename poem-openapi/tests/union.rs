use poem_openapi::{
    registry::{
        MetaDiscriminatorObject, MetaExternalDocument, MetaSchema, MetaSchemaRef, Registry,
    },
    types::{ParseFromJSON, ToJSON, Type},
    Object, Union,
};
use serde_json::json;

#[test]
fn with_discriminator() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v3: bool,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(discriminator_name = "type")]
    enum MyObj {
        A(A),
        B(B),
    }

    assert_eq!(
        MyObj::schema_ref(),
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            ty: "object",
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![]
            }),
            any_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    all_of: vec![
                        MetaSchemaRef::Reference("A"),
                        MetaSchemaRef::Inline(Box::new(MetaSchema {
                            title: Some("A".to_string()),
                            properties: vec![(
                                "type",
                                String::schema_ref().merge(MetaSchema {
                                    example: Some("A".into()),
                                    ..MetaSchema::ANY
                                })
                            )],
                            ..MetaSchema::new("object")
                        })),
                    ],
                    ..MetaSchema::ANY
                })),
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    all_of: vec![
                        MetaSchemaRef::Reference("B"),
                        MetaSchemaRef::Inline(Box::new(MetaSchema {
                            title: Some("B".to_string()),
                            properties: vec![(
                                "type",
                                String::schema_ref().merge(MetaSchema {
                                    example: Some("B".into()),
                                    ..MetaSchema::ANY
                                })
                            )],
                            ..MetaSchema::new("object")
                        })),
                    ],
                    ..MetaSchema::ANY
                }))
            ],
            ..MetaSchema::ANY
        }))
    );

    let mut registry = Registry::new();
    MyObj::register(&mut registry);
    assert!(registry.schemas.contains_key("A"));
    assert!(registry.schemas.contains_key("B"));

    assert_eq!(
        MyObj::parse_from_json(json!({
            "type": "A",
            "v1": 100,
            "v2": "hello",
        }))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
        .to_json(),
        json!({
            "type": "A",
            "v1": 100,
            "v2": "hello",
        })
    );

    assert_eq!(
        MyObj::parse_from_json(json!({
            "type": "B",
            "v3": true,
        }))
        .unwrap(),
        MyObj::B(B { v3: true })
    );

    assert_eq!(
        MyObj::B(B { v3: true }).to_json(),
        json!({
            "type": "B",
            "v3": true,
        })
    );
}

#[test]
fn with_discriminator_mapping() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v3: bool,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(discriminator_name = "type")]
    enum MyObj {
        #[oai(mapping = "c")]
        A(A),
        #[oai(mapping = "d")]
        B(B),
    }

    assert_eq!(
        MyObj::schema_ref(),
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            ty: "object",
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![
                    ("c", "#/components/schemas/A".to_string()),
                    ("d", "#/components/schemas/B".to_string()),
                ]
            }),
            any_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    all_of: vec![
                        MetaSchemaRef::Reference("A"),
                        MetaSchemaRef::Inline(Box::new(MetaSchema {
                            title: Some("c".to_string()),
                            properties: vec![(
                                "type",
                                String::schema_ref().merge(MetaSchema {
                                    example: Some("c".into()),
                                    ..MetaSchema::ANY
                                })
                            )],
                            ..MetaSchema::new("object")
                        })),
                    ],
                    ..MetaSchema::ANY
                })),
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    all_of: vec![
                        MetaSchemaRef::Reference("B"),
                        MetaSchemaRef::Inline(Box::new(MetaSchema {
                            title: Some("d".to_string()),
                            properties: vec![(
                                "type",
                                String::schema_ref().merge(MetaSchema {
                                    example: Some("d".into()),
                                    ..MetaSchema::ANY
                                })
                            )],
                            ..MetaSchema::new("object")
                        })),
                    ],
                    ..MetaSchema::ANY
                }))
            ],
            ..MetaSchema::ANY
        }))
    );

    let mut registry = Registry::new();
    MyObj::register(&mut registry);
    assert!(registry.schemas.contains_key("A"));
    assert!(registry.schemas.contains_key("B"));

    assert_eq!(
        MyObj::parse_from_json(json!({
            "type": "c",
            "v1": 100,
            "v2": "hello",
        }))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
        .to_json(),
        json!({
            "type": "c",
            "v1": 100,
            "v2": "hello",
        })
    );

    assert_eq!(
        MyObj::parse_from_json(json!({
            "type": "d",
            "v3": true,
        }))
        .unwrap(),
        MyObj::B(B { v3: true })
    );

    assert_eq!(
        MyObj::B(B { v3: true }).to_json(),
        json!({
            "type": "d",
            "v3": true,
        })
    );
}

#[test]
fn without_discriminator() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Union, Debug, PartialEq)]
    enum MyObj {
        A(A),
        B(bool),
    }

    assert_eq!(
        MyObj::schema_ref(),
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            ty: "object",
            discriminator: None,
            any_of: vec![MetaSchemaRef::Reference("A"), bool::schema_ref()],
            ..MetaSchema::ANY
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(json!({
            "v1": 100,
            "v2": "hello",
        }))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
        .to_json(),
        json!({
            "v1": 100,
            "v2": "hello",
        })
    );

    assert_eq!(MyObj::parse_from_json(json!(true)).unwrap(), MyObj::B(true));
    assert_eq!(MyObj::B(true).to_json(), json!(true));
}

#[test]
fn anyof() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v1: i32,
    }

    #[derive(Union, Debug, PartialEq)]
    enum MyObj {
        A(A),
        B(B),
    }

    assert_eq!(
        MyObj::schema_ref(),
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            ty: "object",
            discriminator: None,
            any_of: vec![MetaSchemaRef::Reference("A"), MetaSchemaRef::Reference("B")],
            ..MetaSchema::ANY
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(json!({
            "v1": 100,
            "v2": "hello",
        }))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::parse_from_json(json!({
            "v1": 100,
        }))
        .unwrap(),
        MyObj::B(B { v1: 100 })
    );
}

#[test]
fn oneof() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v1: i32,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(one_of)]
    enum MyObj {
        A(A),
        B(B),
    }

    assert_eq!(
        MyObj::schema_ref(),
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            ty: "object",
            discriminator: None,
            one_of: vec![MetaSchemaRef::Reference("A"), MetaSchemaRef::Reference("B")],
            ..MetaSchema::ANY
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(json!({
            "v1": 100,
        }))
        .unwrap(),
        MyObj::B(B { v1: 100 })
    );

    assert_eq!(
        MyObj::parse_from_json(json!({
            "v1": 100,
            "v2": "hello",
        }))
        .unwrap_err()
        .into_message(),
        "Expected input type \"object\", found {\"v1\":100,\"v2\":\"hello\"}."
    );
}

#[test]
fn title_and_description() {
    /// A
    ///
    /// B
    /// C
    #[derive(Union, Debug, PartialEq)]
    enum MyObj2 {
        A(i32),
        B(f32),
    }

    let schema_ref: MetaSchemaRef = MyObj2::schema_ref();
    let meta_schema = schema_ref.unwrap_inline();
    assert_eq!(meta_schema.description, Some("A\n\nB\nC"));
}

#[tokio::test]
async fn external_docs() {
    #[derive(Union, Debug, PartialEq)]
    #[oai(
        external_docs = "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
    )]
    enum MyObj2 {
        A(i32),
        B(f32),
    }

    let schema_ref: MetaSchemaRef = MyObj2::schema_ref();
    let meta_schema = schema_ref.unwrap_inline();
    assert_eq!(
        meta_schema.external_docs,
        Some(MetaExternalDocument {
            url: "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
                .to_string(),
            description: None
        })
    );
}
