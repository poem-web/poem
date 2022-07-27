use poem_openapi::{
    registry::{
        MetaDiscriminatorObject, MetaExternalDocument, MetaSchema, MetaSchemaRef, Registry,
    },
    types::{ParseFromJSON, ToJSON, Type},
    Object, Union,
};
use serde_json::json;

fn get_meta<T: Type>() -> MetaSchema {
    let mut registry = Registry::new();
    T::register(&mut registry);
    registry.schemas.remove(&*T::name()).unwrap()
}

fn get_meta_by_name<T: Type>(name: &str) -> MetaSchema {
    let mut registry = Registry::new();
    T::register(&mut registry);
    registry.schemas.remove(name).unwrap()
}

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

    let schema = get_meta::<MyObj>();

    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::with_discriminator::MyObj"),
            ty: "object",
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![
                    ("A".to_string(), "#/components/schemas/MyObj_A".to_string()),
                    ("B".to_string(), "#/components/schemas/MyObj_B".to_string()),
                ]
            }),
            any_of: vec![
                MetaSchemaRef::Reference("MyObj_A".to_string()),
                MetaSchemaRef::Reference("MyObj_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_a = get_meta_by_name::<MyObj>("MyObj_A");
    assert_eq!(
        schema_myobj_a,
        MetaSchema {
            all_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    properties: vec![(
                        "type",
                        String::schema_ref().merge(MetaSchema {
                            example: Some("A".into()),
                            ..MetaSchema::ANY
                        }),
                    )],
                    ..MetaSchema::new("object")
                })),
                MetaSchemaRef::Reference("A".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_b = get_meta_by_name::<MyObj>("MyObj_B");
    assert_eq!(
        schema_myobj_b,
        MetaSchema {
            all_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    properties: vec![(
                        "type",
                        String::schema_ref().merge(MetaSchema {
                            example: Some("B".into()),
                            ..MetaSchema::ANY
                        })
                    )],
                    ..MetaSchema::new("object")
                })),
                MetaSchemaRef::Reference("B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "A",
            "v1": 100,
            "v2": "hello",
        })))
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
        Some(json!({
            "type": "A",
            "v1": 100,
            "v2": "hello",
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "B",
            "v3": true,
        })))
        .unwrap(),
        MyObj::B(B { v3: true })
    );

    assert_eq!(
        MyObj::B(B { v3: true }).to_json(),
        Some(json!({
            "type": "B",
            "v3": true,
        }))
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

    let schema = get_meta::<MyObj>();

    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::with_discriminator_mapping::MyObj"),
            ty: "object",
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![
                    ("c".to_string(), "#/components/schemas/MyObj_A".to_string()),
                    ("d".to_string(), "#/components/schemas/MyObj_B".to_string()),
                ]
            }),
            any_of: vec![
                MetaSchemaRef::Reference("MyObj_A".to_string()),
                MetaSchemaRef::Reference("MyObj_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_a = get_meta_by_name::<MyObj>("MyObj_A");
    assert_eq!(
        schema_myobj_a,
        MetaSchema {
            all_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    properties: vec![(
                        "type",
                        String::schema_ref().merge(MetaSchema {
                            example: Some("c".into()),
                            ..MetaSchema::ANY
                        }),
                    )],
                    ..MetaSchema::new("object")
                })),
                MetaSchemaRef::Reference("A".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_b = get_meta_by_name::<MyObj>("MyObj_B");
    assert_eq!(
        schema_myobj_b,
        MetaSchema {
            all_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    properties: vec![(
                        "type",
                        String::schema_ref().merge(MetaSchema {
                            example: Some("d".into()),
                            ..MetaSchema::ANY
                        })
                    )],
                    ..MetaSchema::new("object")
                })),
                MetaSchemaRef::Reference("B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let mut registry = Registry::new();
    MyObj::register(&mut registry);
    assert!(registry.schemas.contains_key("A"));
    assert!(registry.schemas.contains_key("B"));

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "c",
            "v1": 100,
            "v2": "hello",
        })))
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
        Some(json!({
            "type": "c",
            "v1": 100,
            "v2": "hello",
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "d",
            "v3": true,
        })))
        .unwrap(),
        MyObj::B(B { v3: true })
    );

    assert_eq!(
        MyObj::B(B { v3: true }).to_json(),
        Some(json!({
            "type": "d",
            "v3": true,
        }))
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

    let schema = get_meta::<MyObj>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::without_discriminator::MyObj"),
            ty: "object",
            discriminator: None,
            any_of: vec![
                MetaSchemaRef::Reference("A".to_string()),
                bool::schema_ref()
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
            "v2": "hello",
        })))
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
        Some(json!({
            "v1": 100,
            "v2": "hello",
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!(true))).unwrap(),
        MyObj::B(true)
    );
    assert_eq!(MyObj::B(true).to_json(), Some(json!(true)));
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

    let schema = get_meta::<MyObj>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::anyof::MyObj"),
            ty: "object",
            discriminator: None,
            any_of: vec![
                MetaSchemaRef::Reference("A".to_string()),
                MetaSchemaRef::Reference("B".to_string())
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
            "v2": "hello",
        })))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
        })))
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

    let schema = get_meta::<MyObj>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::oneof::MyObj"),
            ty: "object",
            discriminator: None,
            one_of: vec![
                MetaSchemaRef::Reference("A".to_string()),
                MetaSchemaRef::Reference("B".to_string())
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
        })))
        .unwrap(),
        MyObj::B(B { v1: 100 })
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
            "v2": "hello",
        })))
        .unwrap_err()
        .into_message(),
        "Expected input type \"MyObj\", found {\"v1\":100,\"v2\":\"hello\"}."
    );
}

#[test]
fn title_and_description() {
    /// A
    ///
    /// B
    /// C
    #[derive(Union, Debug, PartialEq)]
    enum MyObj {
        A(i32),
        B(f32),
    }

    let schema = get_meta::<MyObj>();
    assert_eq!(schema.description, Some("A\n\nB\nC"));
}

#[tokio::test]
async fn external_docs() {
    #[derive(Union, Debug, PartialEq)]
    #[oai(
        external_docs = "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
    )]
    enum MyObj {
        A(i32),
        B(f32),
    }

    let schema = get_meta::<MyObj>();
    assert_eq!(
        schema.external_docs,
        Some(MetaExternalDocument {
            url: "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
                .to_string(),
            description: None
        })
    );
}

#[tokio::test]
async fn generics() {
    #[derive(Union, Debug, PartialEq)]
    enum MyObj<A: ParseFromJSON + ToJSON, B: ParseFromJSON + ToJSON> {
        A(A),
        B(B),
    }

    let schema_i32_i64 = get_meta::<MyObj<i32, i64>>();
    let schema_f32_f64 = get_meta::<MyObj<f32, f64>>();

    assert_eq!(
        <MyObj<i32, i64>>::schema_ref(),
        MetaSchemaRef::Reference("MyObj<integer(int32), integer(int64)>".to_string())
    );
    assert_eq!(
        <MyObj<f32, f64>>::schema_ref(),
        MetaSchemaRef::Reference("MyObj<number(float), number(double)>".to_string())
    );

    assert_eq!(schema_i32_i64.any_of[0], i32::schema_ref());
    assert_eq!(schema_i32_i64.any_of[1], i64::schema_ref());

    assert_eq!(schema_f32_f64.any_of[0], f32::schema_ref());
    assert_eq!(schema_f32_f64.any_of[1], f64::schema_ref());
}

#[test]
fn rename_all() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        value: i32,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        value: String,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(discriminator_name = "type", rename_all = "camelCase")]
    enum MyObj {
        PutInt(A),
        PutString(B),
    }

    let schema = get_meta::<MyObj>();

    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::rename_all::MyObj"),
            ty: "object",
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![
                    (
                        "putInt".to_string(),
                        "#/components/schemas/MyObj_A".to_string()
                    ),
                    (
                        "putString".to_string(),
                        "#/components/schemas/MyObj_B".to_string()
                    ),
                ]
            }),
            any_of: vec![
                MetaSchemaRef::Reference("MyObj_A".to_string()),
                MetaSchemaRef::Reference("MyObj_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "putInt",
            "value": 100,
        })))
        .unwrap(),
        MyObj::PutInt(A { value: 100 })
    );

    assert_eq!(
        MyObj::PutInt(A { value: 100 }).to_json(),
        Some(json!({
            "type": "putInt",
            "value": 100,
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "putString",
            "value": "abc",
        })))
        .unwrap(),
        MyObj::PutString(B {
            value: "abc".to_string()
        })
    );

    assert_eq!(
        MyObj::PutString(B {
            value: "abc".to_string()
        })
        .to_json(),
        Some(json!({
            "type": "putString",
            "value": "abc",
        }))
    );
}
