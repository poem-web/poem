use std::ops::Range;

use poem::{http::StatusCode, test::TestClient};
use poem_openapi::{
    Object, OpenApi, OpenApiService,
    payload::Json,
    registry::{MetaSchemaRef, Registry},
    types::{MaybeUndefined, ParseFromJSON, ToJSON, Type},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[test]
fn range_type_and_schema() {
    const { assert!(<Range<i32>>::IS_REQUIRED) };
    assert_eq!(<Range<i32>>::name(), "range_integer_int32");
    assert_ne!(<Range<i32>>::name(), <Range<String>>::name());

    let range = 1..5;
    assert_eq!(range.as_raw_value(), Some(&range));
    assert_eq!(range.raw_element_iter().collect::<Vec<_>>(), [&range]);
    assert_eq!(
        serde_json::to_value(<Range<i32>>::schema_ref()).unwrap(),
        json!({
            "type": "object",
            "description": "A half-open range with an inclusive start and exclusive end.",
            "required": ["start", "end"],
            "properties": {
                "start": {"type": "integer", "format": "int32"},
                "end": {"type": "integer", "format": "int32"},
            },
            "maxProperties": 2,
        }),
    );
}

#[test]
fn object_parsing_matches_serde() {
    for value in [
        json!({"start": 1, "end": 5}),
        json!({"start": 5, "end": 5}),
        json!({"start": 5, "end": 1}),
        json!({"start": -10, "end": -2}),
        json!({"start": i32::MIN, "end": i32::MAX}),
        json!({"start": i64::from(i32::MIN) - 1, "end": 5}),
        json!({"start": 1, "end": i64::from(i32::MAX) + 1}),
        json!({}),
        json!({"start": 1}),
        json!({"end": 5}),
        json!({"start": null, "end": 5}),
        json!({"start": 1, "end": null}),
        json!({"start": "1", "end": 5}),
        json!({"start": 1, "end": "5"}),
        json!({"start": 1.5, "end": 5}),
        json!({"start": 1, "end": 5, "extra": true}),
        json!(null),
        json!(true),
        json!(5),
        json!("1..5"),
    ] {
        let actual = <Range<i32>>::parse_from_json(Some(value.clone()));
        let expected = serde_json::from_value::<Range<i32>>(value.clone());
        match (actual, expected) {
            (Ok(actual), Ok(expected)) => {
                assert_eq!(actual, expected);
                assert_eq!(
                    actual.to_json(),
                    Some(serde_json::to_value(expected).unwrap())
                );
            }
            (Err(_), Err(_)) => {}
            (actual, expected) => panic!("parsers disagree for {value}: {actual:?}, {expected:?}"),
        }
    }
}

#[test]
fn arrays_are_not_a_range_encoding() {
    // Serde also accepts a two-element sequence. Poem's JSON contract is the
    // object representation described by its OpenAPI schema.
    assert_eq!(
        serde_json::from_value::<Range<i32>>(json!([1, 5])).unwrap(),
        1..5
    );
    for value in [json!([]), json!([1]), json!([1, 5]), json!([1, 5, 9])] {
        assert!(<Range<i32>>::parse_from_json(Some(value)).is_err());
    }
}

#[test]
fn errors_identify_missing_or_invalid_endpoints() {
    assert!(
        <Range<i32>>::parse_from_json(None)
            .unwrap_err()
            .message()
            .contains("expects an input value")
    );
    for (value, message) in [
        (json!({"end": 5}), "missing field `start`"),
        (json!({"start": 1}), "missing field `end`"),
        (json!({"start": false, "end": 5}), "field `start`"),
        (json!({"start": 1, "end": false}), "field `end`"),
        (
            json!({"start": 1, "end": 5, "extra": true}),
            "unknown field `extra`",
        ),
    ] {
        let error = <Range<i32>>::parse_from_json(Some(value)).unwrap_err();
        assert!(error.message().contains(message), "{}", error.message());
    }
}

#[test]
fn optional_endpoints_require_keys_and_preserve_nulls() {
    for value in [
        json!({"start": null, "end": null}),
        json!({"start": null, "end": 5}),
        json!({"start": 1, "end": null}),
        json!({"start": 1, "end": 5}),
        json!({}),
        json!({"start": null}),
        json!({"end": null}),
        json!({"start": null, "end": null, "extra": true}),
    ] {
        let actual = <Range<Option<i32>>>::parse_from_json(Some(value.clone()));
        let expected = serde_json::from_value::<Range<Option<i32>>>(value.clone());
        match (actual, expected) {
            (Ok(actual), Ok(expected)) => {
                assert_eq!(actual, expected);
                assert_eq!(actual.to_json(), Some(value));
            }
            (Err(_), Err(_)) => {}
            (actual, expected) => panic!("parsers disagree for {value}: {actual:?}, {expected:?}"),
        }
    }
    let schema = serde_json::to_value(<Range<Option<i32>>>::schema_ref()).unwrap();
    assert_eq!(schema["required"], json!(["start", "end"]));
    for field in ["start", "end"] {
        assert_eq!(
            schema["properties"][field],
            json!({
                "type": "integer", "format": "int32", "nullable": true,
            })
        );
    }
    assert_eq!(
        (MaybeUndefined::<i32>::Undefined..MaybeUndefined::Null).to_json(),
        Some(json!({"start": null, "end": null}))
    );
}

#[derive(Debug, PartialEq, Object, Serialize, Deserialize)]
struct Bound {
    value: i32,
}

#[derive(Debug, PartialEq, Object)]
struct Bounds {
    bounds: Range<Bound>,
    optional: Range<Option<Bound>>,
    nested: Vec<Range<Range<i32>>>,
}

#[test]
fn derived_and_nested_endpoints() {
    let value = json!({
        "bounds": {"start": {"value": 1}, "end": {"value": 5}},
        "optional": {"start": null, "end": {"value": 9}},
        "nested": [{"start": {"start": 0, "end": 1}, "end": {"start": 2, "end": 3}}],
    });
    let bounds = Bounds::parse_from_json(Some(value.clone())).unwrap();
    assert_eq!(bounds.bounds, Bound { value: 1 }..Bound { value: 5 });
    assert_eq!(bounds.optional, None..Some(Bound { value: 9 }));
    assert_eq!(bounds.to_json(), Some(value));
    assert_eq!(
        bounds.bounds.to_json(),
        Some(serde_json::to_value(&bounds.bounds).unwrap())
    );

    let mut registry = Registry::new();
    Bounds::register(&mut registry);
    assert!(registry.schemas.contains_key("Bound"));
    let required_schema = serde_json::to_value(<Range<Bound>>::schema_ref()).unwrap();
    let optional_schema = serde_json::to_value(<Range<Option<Bound>>>::schema_ref()).unwrap();
    for field in ["start", "end"] {
        assert_eq!(
            required_schema["properties"][field],
            json!({"$ref": "#/components/schemas/Bound"})
        );
        assert_eq!(
            optional_schema["properties"][field]["anyOf"],
            json!([
                {"$ref": "#/components/schemas/Bound"},
                {"type": "object", "nullable": true, "enum": [null]},
            ])
        );
    }
    // Making a range endpoint nullable must not weaken the shared component.
    assert!(!registry.schemas["Bound"].nullable);
}

#[test]
fn endpoints_need_no_serde_or_ordering_traits() {
    #[derive(Debug, PartialEq, Object)]
    struct Endpoint {
        name: String,
    }
    let value = json!({"start": {"name": "z"}, "end": {"name": "a"}});
    let range = <Range<Endpoint>>::parse_from_json(Some(value.clone())).unwrap();
    assert_eq!(range.to_json(), Some(value));
    let strings = String::from("z")..String::from("a");
    assert_eq!(
        strings.to_json(),
        Some(serde_json::to_value(strings).unwrap())
    );
    assert_eq!(
        (()..()).to_json(),
        Some(json!({"start": null, "end": null}))
    );
}

#[test]
fn range_schemas_do_not_collide_for_shared_endpoint_names() {
    #[derive(Object)]
    struct Ranges {
        unsigned: Range<u64>,
        size: Range<usize>,
        pair: Range<[i32; 2]>,
        triple: Range<[i32; 3]>,
        optional: Range<Option<i32>>,
        undefined: Range<MaybeUndefined<i32>>,
        boxed: Range<Box<Bound>>,
        plain: Range<Bound>,
    }
    let mut registry = Registry::new();
    Ranges::register(&mut registry);
    assert_eq!(registry.schemas.len(), 2);
    assert!(matches!(
        <Range<u64>>::schema_ref(),
        MetaSchemaRef::Inline(_)
    ));
    let schema = serde_json::to_value(&registry.schemas["Ranges"]).unwrap();
    assert_eq!(
        schema["properties"]["pair"]["properties"]["start"]["minItems"],
        2
    );
    assert_eq!(
        schema["properties"]["triple"]["properties"]["start"]["minItems"],
        3
    );
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/range", method = "post")]
    async fn range(&self, value: Json<Range<i32>>) -> Json<Range<i32>> {
        value
    }

    #[oai(path = "/bounds", method = "post")]
    async fn bounds(&self, value: Json<Bounds>) -> Json<Bounds> {
        value
    }
}

#[tokio::test]
async fn json_endpoints_and_generated_spec() {
    let service = OpenApiService::new(Api, "Ranges", "1.0");
    let spec: Value = serde_json::from_str(&service.spec()).unwrap();
    assert_eq!(spec["openapi"], "3.0.0");
    let operation = &spec["paths"]["/range"]["post"];
    let schema = serde_json::to_value(<Range<i32>>::schema_ref()).unwrap();
    assert_eq!(operation["requestBody"]["required"], true);
    assert_eq!(
        operation["requestBody"]["content"]["application/json; charset=utf-8"]["schema"],
        schema
    );
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json; charset=utf-8"]["schema"],
        schema
    );
    assert_eq!(spec["components"]["schemas"]["Bound"]["type"], "object");
    assert_eq!(
        spec["components"]["schemas"]["Bounds"]["properties"]["optional"]["properties"]["start"]["anyOf"]
            [0],
        json!({"$ref": "#/components/schemas/Bound"})
    );

    let client = TestClient::new(service);
    for value in [
        json!({"start": 1, "end": 5}),
        json!({"start": 5, "end": 5}),
        json!({"start": 5, "end": 1}),
    ] {
        let response = client.post("/range").body_json(&value).send().await;
        response.assert_status_is_ok();
        response.assert_json(value).await;
    }
    for value in [
        json!(null),
        json!([1, 5]),
        json!({"start": 1}),
        json!({"start": 1, "end": false}),
        json!({"start": 1, "end": 5, "extra": true}),
    ] {
        client
            .post("/range")
            .body_json(&value)
            .send()
            .await
            .assert_status(StatusCode::BAD_REQUEST);
    }
    let value = json!({
        "bounds": {"start": {"value": 1}, "end": {"value": 5}},
        "optional": {"start": null, "end": {"value": 9}},
        "nested": [],
    });
    let response = client.post("/bounds").body_json(&value).send().await;
    response.assert_status_is_ok();
    response.assert_json(value).await;
}
