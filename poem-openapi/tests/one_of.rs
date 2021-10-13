use poem_openapi::{
    registry::{MetaDiscriminatorObject, MetaSchema, MetaSchemaRef, Registry},
    types::{ParseFromJSON, ToJSON, Type},
    Object, OneOf,
};
use serde_json::json;

#[derive(Object, Debug, PartialEq)]
struct A {
    v1: i32,
    v2: String,
}

#[derive(Object, Debug, PartialEq)]
struct B {
    v3: f32,
}

#[derive(OneOf, Debug, PartialEq)]
#[oai(property_name = "type")]
enum MyObj {
    A(A),
    B(B),
}

#[test]
fn one_of_meta() {
    assert_eq!(
        MyObj::schema_ref(),
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            properties: vec![(
                "type",
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    enum_items: vec!["A".into(), "B".into()],
                    ..MetaSchema::new("string")
                }))
            )],
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![]
            }),
            one_of: vec![MetaSchemaRef::Reference("A"), MetaSchemaRef::Reference("B")],
            ..MetaSchema::new("object")
        }))
    );

    let mut registry = Registry::new();
    MyObj::register(&mut registry);
    assert!(registry.schemas.contains_key("A"));
    assert!(registry.schemas.contains_key("B"));
}

#[test]
fn one_of_serialize() {
    assert_eq!(
        MyObj::parse_from_json(json! ({
            "v1": 100,
            "v2": "abc".to_string(),
        }))
        .unwrap_err()
        .into_message(),
        r#"Expected input type "object", found {"v1":100,"v2":"abc"}."#
    );

    assert_eq!(
        MyObj::parse_from_json(json! ({
            "type": "A",
            "v1": 100,
            "v2": "abc".to_string(),
        }))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "abc".to_string()
        })
    );

    assert_eq!(
        MyObj::parse_from_json(json! ({
            "type": "B",
            "v3": 99,
        }))
        .unwrap(),
        MyObj::B(B { v3: 99.0 })
    );

    assert_eq!(
        MyObj::A(A {
            v1: 100,
            v2: "abc".to_string()
        })
        .to_json(),
        json!({
            "type": "A",
            "v1": 100,
            "v2": "abc",
        })
    );

    assert_eq!(
        MyObj::B(B { v3: 88.0 }).to_json(),
        json!({
            "type": "B",
            "v3": 88.0,
        })
    );
}

#[test]
fn mapping() {
    #[derive(Object, Debug, PartialEq)]
    struct Abc {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct Def {
        v3: f32,
    }

    #[derive(OneOf, Debug, PartialEq)]
    #[oai(property_name = "type")]
    enum MyOneOf {
        #[oai(mapping = "a1")]
        A(Abc),
        #[oai(mapping = "a2")]
        B(Def),
    }

    assert_eq!(
        MyOneOf::schema_ref(),
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            properties: vec![(
                "type",
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    enum_items: vec!["a1".into(), "a2".into()],
                    ..MetaSchema::new("string")
                }))
            )],
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![
                    ("a1", "#/components/schemas/Abc".to_string()),
                    ("a2", "#/components/schemas/Def".to_string())
                ],
            }),
            one_of: vec![
                MetaSchemaRef::Reference("Abc"),
                MetaSchemaRef::Reference("Def")
            ],
            ..MetaSchema::new("object")
        }))
    );

    let mut registry = Registry::new();
    MyOneOf::register(&mut registry);
    assert!(registry.schemas.contains_key("Abc"));
    assert!(registry.schemas.contains_key("Def"));

    assert_eq!(
        MyOneOf::parse_from_json(json! ({
            "type": "a1",
            "v1": 99,
            "v2": "abc".to_string(),
        }))
        .unwrap(),
        MyOneOf::A(Abc {
            v1: 99,
            v2: "abc".to_string()
        })
    );

    assert_eq!(
        MyOneOf::parse_from_json(json! ({
            "type": "a2",
            "v3": 99,
        }))
        .unwrap(),
        MyOneOf::B(Def { v3: 99.0 })
    );

    assert_eq!(
        MyOneOf::B(Def { v3: 88.0 }).to_json(),
        json!({
            "type": "a2",
            "v3": 88.0,
        })
    );
}
