use std::collections::BTreeMap;

use poem::{http::StatusCode, test::TestClient};
use poem_openapi::{
    Enum, Object, OpenApi, OpenApiService, Union,
    payload::Json,
    registry::{MetaSchema, MetaSchemaRef, Registry},
    types::{MaybeUndefined, ParseFromJSON, ToJSON, Type},
};
use serde_json::{Value, json};

fn schema<T: Type>() -> Value {
    serde_json::to_value(T::schema_ref()).unwrap()
}

fn nullable_ref(name: &str) -> Value {
    json!({"anyOf": [
        {"$ref": format!("#/components/schemas/{name}")},
        {"type": "object", "nullable": true, "enum": [null]},
    ]})
}

#[derive(Debug, PartialEq, Object)]
struct Child {
    value: String,
}

#[derive(Debug, PartialEq, Enum)]
enum Choice {
    First,
    Second,
}

#[derive(Debug, PartialEq, Object)]
struct Count {
    count: i32,
}

#[derive(Debug, PartialEq, Union)]
#[oai(one_of)]
enum Either {
    Text(Child),
    Count(Count),
}

#[test]
fn optional_schemas_describe_null_without_changing_requiredness() {
    const { assert!(!Option::<String>::IS_REQUIRED) };
    const { assert!(!MaybeUndefined::<String>::IS_REQUIRED) };
    const { assert!(String::IS_REQUIRED) };
    let string = json!({"type": "string", "nullable": true});
    assert_eq!(schema::<Option<String>>(), string);
    assert_eq!(schema::<MaybeUndefined<String>>(), string);
    assert_eq!(schema::<Option<Option<String>>>(), string);
    assert_eq!(schema::<Option<MaybeUndefined<String>>>(), string);
    assert_eq!(schema::<MaybeUndefined<Option<String>>>(), string);
    assert_eq!(
        schema::<Option<i32>>(),
        json!({"type": "integer", "format": "int32", "nullable": true})
    );
    assert_eq!(
        schema::<Option<bool>>(),
        json!({"type": "boolean", "nullable": true})
    );
    assert_eq!(schema::<Option<()>>(), schema::<()>());
}

#[test]
fn referenced_schemas_are_nullable_without_weakening_components() {
    for (actual, name) in [
        (schema::<Option<Child>>(), "Child"),
        (schema::<MaybeUndefined<Child>>(), "Child"),
        (schema::<Option<Option<Child>>>(), "Child"),
        (schema::<MaybeUndefined<Option<Child>>>(), "Child"),
        (schema::<Option<Choice>>(), "Choice"),
        (schema::<Option<Either>>(), "Either"),
    ] {
        assert_eq!(actual, nullable_ref(name));
    }
    let mut registry = Registry::new();
    Option::<Child>::register(&mut registry);
    MaybeUndefined::<Choice>::register(&mut registry);
    Option::<Either>::register(&mut registry);
    for name in ["Child", "Choice", "Either"] {
        assert!(!registry.schemas[name].nullable);
        assert!(!registry.schemas[name].enum_items.contains(&Value::Null));
    }
    assert_eq!(registry.schemas["Child"].required, ["value"]);
    assert_eq!(registry.schemas["Either"].one_of.len(), 2);
}

#[test]
fn nullable_collections_preserve_item_schemas_and_validators() {
    #[derive(Object)]
    struct Collections {
        #[oai(validator(max_items = 2, max_length = 3))]
        list: Option<Vec<String>>,
        #[oai(validator(max_properties = 2, max_length = 3))]
        map: Option<BTreeMap<String, String>>,
        nested: Vec<Option<Vec<Option<Child>>>>,
    }
    let mut registry = Registry::new();
    Collections::register(&mut registry);
    let value = serde_json::to_value(&registry.schemas["Collections"]).unwrap();
    assert_eq!(value["required"], json!(["nested"]));
    assert_eq!(
        value["properties"]["list"],
        json!({
            "type": "array", "nullable": true, "maxItems": 2,
            "items": {"type": "string", "maxLength": 3},
        })
    );
    assert_eq!(
        value["properties"]["map"],
        json!({
            "type": "object", "nullable": true, "maxProperties": 2,
            "additionalProperties": {"type": "string", "maxLength": 3},
        })
    );
    assert_eq!(
        value["properties"]["nested"],
        json!({
            "type": "array", "items": {
                "type": "array", "nullable": true, "items": nullable_ref("Child"),
            },
        })
    );
    assert_eq!(
        schema::<Option<[i32; 2]>>(),
        json!({
            "type": "array", "nullable": true, "minItems": 2, "maxItems": 2,
            "items": {"type": "integer", "format": "int32"},
        })
    );
}

#[test]
fn nullable_attributes_compose_with_automatic_nullability() {
    #[derive(Object)]
    struct Explicit {
        #[oai(nullable)]
        required: String,
        #[oai(nullable)]
        optional: Option<Child>,
        #[oai(nullable)]
        choice: Choice,
    }
    #[derive(Object)]
    #[oai(nullable_all)]
    struct All {
        required: String,
        optional: MaybeUndefined<Child>,
        either: Either,
    }
    let mut registry = Registry::new();
    Explicit::register(&mut registry);
    All::register(&mut registry);
    let explicit = serde_json::to_value(&registry.schemas["Explicit"]).unwrap();
    let all = serde_json::to_value(&registry.schemas["All"]).unwrap();
    assert_eq!(explicit["required"], json!(["required", "choice"]));
    assert_eq!(all["required"], json!(["required", "either"]));
    for value in [explicit, all] {
        assert_eq!(
            value["properties"]["required"],
            json!({"type": "string", "nullable": true})
        );
        assert_eq!(value["properties"]["optional"], nullable_ref("Child"));
    }
    assert_eq!(
        serde_json::to_value(&registry.schemas["Explicit"]).unwrap()["properties"]["choice"],
        nullable_ref("Choice")
    );
    assert_eq!(
        serde_json::to_value(&registry.schemas["All"]).unwrap()["properties"]["either"],
        nullable_ref("Either")
    );
    // These schema-only attributes do not change the parser or required fields.
    assert!(Explicit::parse_from_json(Some(json!({"required": null, "choice": "First"}))).is_err());
}

#[test]
fn inline_enums_and_compositions_allow_only_an_additional_null() {
    let nullable = || MetaSchema {
        nullable: true,
        ..MetaSchema::ANY
    };
    let enum_schema = MetaSchemaRef::Inline(Box::new(MetaSchema {
        enum_items: vec![json!("first"), json!("second")],
        ..MetaSchema::new("string")
    }));
    assert_eq!(
        serde_json::to_value(enum_schema.merge(nullable())).unwrap(),
        json!({
            "type": "string", "nullable": true, "enum": ["first", "second", null],
        })
    );
    for keyword in ["oneOf", "anyOf", "allOf"] {
        let mut composed = MetaSchema::new("string");
        let branches = vec![String::schema_ref()];
        match keyword {
            "oneOf" => composed.one_of = branches,
            "anyOf" => composed.any_of = branches,
            _ => composed.all_of = branches,
        }
        let original = serde_json::to_value(&composed).unwrap();
        let actual = MetaSchemaRef::Inline(Box::new(composed)).merge(nullable());
        assert_eq!(
            serde_json::to_value(actual.clone()).unwrap(),
            json!({"anyOf": [
                original, {"type": "object", "nullable": true, "enum": [null]},
            ]})
        );
        assert_eq!(actual.clone().merge(nullable()), actual);
    }
}

#[test]
fn existing_null_branch_does_not_bypass_outer_constraints() {
    for mut restricted in [
        MetaSchema::new("string"),
        MetaSchema {
            enum_items: vec![json!("only")],
            ..MetaSchema::ANY
        },
        MetaSchema {
            one_of: vec![String::schema_ref()],
            ..MetaSchema::ANY
        },
        MetaSchema {
            all_of: vec![String::schema_ref()],
            ..MetaSchema::ANY
        },
    ] {
        restricted.any_of = vec![String::schema_ref(), <()>::schema_ref()];
        let original = MetaSchemaRef::Inline(Box::new(restricted));
        let actual = original.clone().merge(MetaSchema {
            nullable: true,
            ..MetaSchema::ANY
        });
        assert_eq!(
            actual.unwrap_inline().any_of,
            vec![original, <()>::schema_ref()]
        );
    }
}

#[test]
fn recursive_components_and_field_annotations_are_preserved() {
    #[derive(Object)]
    struct Node {
        value: String,
        next: Option<Box<Node>>,
    }
    #[derive(Object)]
    struct Annotated {
        /// An optional child.
        #[oai(default, nullable)]
        child: Option<Child>,
    }
    let mut registry = Registry::new();
    Node::register(&mut registry);
    Annotated::register(&mut registry);
    let node = serde_json::to_value(&registry.schemas["Node"]).unwrap();
    assert_eq!(node["required"], json!(["value"]));
    assert_eq!(node["properties"]["next"], nullable_ref("Node"));
    let mut expected = nullable_ref("Child");
    expected["description"] = json!("An optional child.");
    expected["default"] = Value::Null;
    let annotated = serde_json::to_value(&registry.schemas["Annotated"]).unwrap();
    assert_eq!(annotated["properties"]["child"], expected);
    assert_eq!(
        Annotated { child: None }.to_json(),
        Some(json!({"child": null}))
    );
}

#[test]
fn optional_flattened_fields_keep_their_existing_schema_and_json() {
    #[derive(Debug, PartialEq, Object)]
    struct Flattened {
        #[oai(flatten)]
        child: Option<Child>,
        count: i32,
    }
    #[derive(Object)]
    struct UndefinedFlattened {
        #[oai(flatten)]
        child: MaybeUndefined<Child>,
        count: i32,
    }
    #[derive(Object)]
    struct NestedFlattened {
        #[oai(flatten)]
        child: Option<Option<Child>>,
        count: i32,
    }
    let mut registry = Registry::new();
    NestedFlattened::register(&mut registry);
    Flattened::register(&mut registry);
    UndefinedFlattened::register(&mut registry);
    for name in ["Flattened", "UndefinedFlattened", "NestedFlattened"] {
        let value = serde_json::to_value(&registry.schemas[name]).unwrap();
        assert_eq!(value["required"], json!(["value", "count"]));
        assert_eq!(
            value["properties"],
            json!({
                "value": {"type": "string"}, "count": {"type": "integer", "format": "int32"},
            })
        );
    }
    let value = json!({"value": "hello", "count": 3});
    assert_eq!(
        Flattened::parse_from_json(Some(value.clone()))
            .unwrap()
            .to_json(),
        Some(value)
    );
    assert_eq!(
        Flattened {
            child: None,
            count: 3
        }
        .to_json(),
        Some(json!({"count": 3}))
    );
}

#[derive(Debug, PartialEq, Object)]
struct Values {
    optional: Option<String>,
    nested: Option<Option<String>>,
    undefined: MaybeUndefined<String>,
}

#[test]
fn missing_null_and_value_json_contract_is_unchanged() {
    for (input, expected, output) in [
        (
            json!({}),
            Values {
                optional: None,
                nested: None,
                undefined: MaybeUndefined::Undefined,
            },
            json!({"optional": null, "nested": null}),
        ),
        (
            json!({"optional": null, "nested": null, "undefined": null}),
            Values {
                optional: None,
                nested: None,
                undefined: MaybeUndefined::Null,
            },
            json!({"optional": null, "nested": null, "undefined": null}),
        ),
        (
            json!({"optional": "a", "nested": "b", "undefined": "c"}),
            Values {
                optional: Some("a".into()),
                nested: Some(Some("b".into())),
                undefined: MaybeUndefined::Value("c".into()),
            },
            json!({"optional": "a", "nested": "b", "undefined": "c"}),
        ),
    ] {
        assert_eq!(Values::parse_from_json(Some(input)).unwrap(), expected);
        assert_eq!(expected.to_json(), Some(output));
    }
    assert_eq!(
        Option::<Option<String>>::Some(None).to_json(),
        Some(Value::Null)
    );
    assert_eq!(
        Option::<Option<String>>::parse_from_json(None).unwrap(),
        None
    );
    assert_eq!(
        Option::<Option<String>>::parse_from_json(Some(Value::Null)).unwrap(),
        None
    );
    // String parsing already accepts numeric and boolean values.
    for (input, expected) in [(json!(1), "1"), (json!(true), "true")] {
        assert_eq!(
            Option::<String>::parse_from_json(Some(input.clone())).unwrap(),
            Some(expected.to_string())
        );
        assert_eq!(
            MaybeUndefined::<String>::parse_from_json(Some(input)).unwrap(),
            MaybeUndefined::Value(expected.to_string())
        );
    }
    for invalid in [json!([]), json!({})] {
        assert!(Option::<String>::parse_from_json(Some(invalid.clone())).is_err());
        assert!(MaybeUndefined::<String>::parse_from_json(Some(invalid)).is_err());
    }
}

#[test]
fn explicit_omission_settings_remain_unchanged() {
    #[derive(Object)]
    #[oai(skip_serializing_if_is_none)]
    struct Skip {
        option: Option<String>,
        nested: Option<Option<String>>,
        undefined: MaybeUndefined<String>,
    }
    assert_eq!(
        Skip {
            option: None,
            nested: None,
            undefined: MaybeUndefined::Undefined
        }
        .to_json(),
        Some(json!({}))
    );
    assert_eq!(
        Skip {
            option: None,
            nested: Some(None),
            undefined: MaybeUndefined::Null
        }
        .to_json(),
        Some(json!({"nested": null}))
    );
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/text", method = "get")]
    async fn text(&self) -> Json<Option<String>> {
        Json(None)
    }

    #[oai(path = "/child", method = "get")]
    async fn child(&self) -> Json<Option<Child>> {
        Json(None)
    }

    #[oai(path = "/optional", method = "post")]
    async fn optional(&self, body: Json<Option<Child>>) -> Json<Option<Child>> {
        body
    }

    #[oai(path = "/maybe", method = "post")]
    async fn maybe(&self, body: Json<MaybeUndefined<Child>>) -> Json<MaybeUndefined<Child>> {
        body
    }

    #[oai(path = "/values", method = "post")]
    async fn values(&self, body: Json<Values>) -> Json<Values> {
        body
    }
}

#[tokio::test]
async fn generated_documents_and_endpoint_bodies_agree() {
    let service = OpenApiService::new(Api, "Nullable", "1.0");
    let document: Value = serde_json::from_str(&service.spec()).unwrap();
    assert_eq!(document["openapi"], "3.0.0");
    assert_eq!(
        document["paths"]["/text"]["get"]["responses"]["200"]["content"]["application/json; charset=utf-8"]
            ["schema"],
        json!({"type": "string", "nullable": true})
    );
    assert_eq!(
        document["paths"]["/child"]["get"]["responses"]["200"]["content"]["application/json; charset=utf-8"]
            ["schema"],
        nullable_ref("Child")
    );
    assert_eq!(
        document["components"]["schemas"]["Values"]["properties"],
        json!({
            "optional": {"type": "string", "nullable": true},
            "nested": {"type": "string", "nullable": true},
            "undefined": {"type": "string", "nullable": true},
        })
    );
    assert!(
        document["components"]["schemas"]["Values"]
            .get("required")
            .is_none()
    );
    assert_eq!(
        document["paths"]["/values"]["post"]["requestBody"]["required"],
        true
    );
    for path in ["/optional", "/maybe"] {
        assert_eq!(
            document["paths"][path]["post"]["requestBody"]["required"],
            false
        );
        assert_eq!(
            document["paths"][path]["post"]["requestBody"]["content"]["application/json; charset=utf-8"]
                ["schema"],
            nullable_ref("Child")
        );
    }
    let client = TestClient::new(service);
    for path in ["/optional", "/maybe"] {
        for body in ["", "null"] {
            let response = client
                .post(path)
                .content_type("application/json")
                .body(body)
                .send()
                .await;
            response.assert_status(StatusCode::OK);
            response.assert_text("null").await;
        }
    }
    for path in ["/text", "/child"] {
        let response = client.get(path).send().await;
        response.assert_status(StatusCode::OK);
        response.assert_text("null").await;
    }
    let response = client.post("/values").body_json(&json!({})).send().await;
    response.assert_status(StatusCode::OK);
    response
        .assert_json(json!({"optional": null, "nested": null}))
        .await;
}
