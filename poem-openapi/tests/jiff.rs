#![cfg(feature = "jiff")]

use jiff::{
    Timestamp, Zoned,
    civil::{Date, DateTime, Time},
};
use poem::{
    http::StatusCode,
    test::{TestClient, TestForm},
};
use poem_openapi::{
    Multipart, Object, OpenApi, OpenApiService,
    param::Query,
    payload::Json,
    registry::{MetaSchema, Registry},
    types::{ParseFromJSON, ToJSON, Type},
};
use serde_json::{Value, json};

// Fixed-offset annotations make these tests independent of the system time zone
// database, while still checking that Zoned preserves its time zone annotation.
const FIELDS: [(&str, &str, &str); 5] = [
    (
        "timestamp",
        "2024-02-29T18:04:56.123456789+05:30",
        "date-time",
    ),
    (
        "zoned",
        "2024-02-29T18:04:56.123456789+05:30[+05:30]",
        "zoned-date-time",
    ),
    (
        "datetime",
        "2024-02-29T12:34:56.123456789",
        "naive-date-time",
    ),
    ("date", "2024-02-29", "naive-date"),
    ("time", "12:34:56.123456789", "naive-time"),
];

#[derive(Debug, Object, Multipart, PartialEq)]
struct RequiredTimes {
    timestamp: Timestamp,
    zoned: Zoned,
    datetime: DateTime,
    date: Date,
    time: Time,
}

#[derive(Debug, Object, Multipart, PartialEq)]
struct OptionalTimes {
    timestamp: Option<Timestamp>,
    zoned: Option<Zoned>,
    datetime: Option<DateTime>,
    date: Option<Date>,
    time: Option<Time>,
}

fn input_json() -> Value {
    Value::Object(
        FIELDS
            .iter()
            .map(|&(name, value, _)| (name.to_string(), json!(value)))
            .collect(),
    )
}

fn output_json() -> Value {
    let mut value = input_json();
    value["timestamp"] = json!("2024-02-29T12:34:56.123456789Z");
    value
}

fn null_json() -> Value {
    Value::Object(
        FIELDS
            .iter()
            .map(|&(name, _, _)| (name.to_string(), Value::Null))
            .collect(),
    )
}

fn assert_properties(schema: &MetaSchema, required: bool) {
    assert_eq!(schema.ty, "object");
    assert_eq!(schema.properties.len(), FIELDS.len());
    let expected_required: Vec<_> = FIELDS
        .iter()
        .filter_map(|&(name, _, _)| required.then_some(name))
        .collect();
    assert_eq!(schema.required, expected_required);

    for ((name, schema), &(expected_name, _, format)) in schema.properties.iter().zip(&FIELDS) {
        assert_eq!(*name, expected_name);
        let schema = schema.unwrap_inline();
        assert_eq!(schema.ty, "string");
        assert_eq!(schema.format, Some(format));
        assert_eq!(schema.nullable, !required);
    }
}

#[test]
fn derived_object_schemas() {
    let mut registry = Registry::new();
    <RequiredTimes as Type>::register(&mut registry);
    <OptionalTimes as Type>::register(&mut registry);
    assert_properties(&registry.schemas["RequiredTimes"], true);
    assert_properties(&registry.schemas["OptionalTimes"], false);
}

#[test]
fn derived_multipart_schemas() {
    let required = <RequiredTimes as poem_openapi::payload::Payload>::schema_ref();
    let optional = <OptionalTimes as poem_openapi::payload::Payload>::schema_ref();
    assert_properties(required.unwrap_inline(), true);
    assert_properties(optional.unwrap_inline(), false);
}

#[test]
fn object_json_roundtrip() {
    let required = RequiredTimes::parse_from_json(Some(input_json())).unwrap();
    assert_eq!(required.to_json(), Some(output_json()));
    assert_eq!(
        RequiredTimes::parse_from_json(required.to_json()).unwrap(),
        required
    );

    let optional = OptionalTimes::parse_from_json(Some(input_json())).unwrap();
    assert_eq!(optional.to_json(), Some(output_json()));
    assert_eq!(
        OptionalTimes::parse_from_json(optional.to_json()).unwrap(),
        optional
    );

    let absent = OptionalTimes::parse_from_json(Some(json!({}))).unwrap();
    let null = OptionalTimes::parse_from_json(Some(null_json())).unwrap();
    assert_eq!(absent, null);
    assert_eq!(absent.to_json(), Some(null_json()));
}

#[test]
fn object_json_rejects_missing_and_invalid_fields() {
    for (name, _, _) in FIELDS {
        let mut missing = input_json();
        missing.as_object_mut().unwrap().remove(name);
        assert!(RequiredTimes::parse_from_json(Some(missing)).is_err());

        for invalid in [
            Value::Null,
            json!(42),
            json!(false),
            json!([]),
            json!({}),
            json!("not-a-date"),
            json!(""),
        ] {
            let mut value = input_json();
            value[name] = invalid.clone();
            assert!(
                RequiredTimes::parse_from_json(Some(value.clone())).is_err(),
                "required {name} accepted {invalid}"
            );
            if !invalid.is_null() {
                assert!(
                    OptionalTimes::parse_from_json(Some(value)).is_err(),
                    "optional {name} accepted {invalid}"
                );
            }
        }
    }
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/query", method = "get")]
    async fn query(
        &self,
        timestamp: Query<Timestamp>,
        zoned: Query<Zoned>,
        datetime: Query<DateTime>,
        date: Query<Date>,
        time: Query<Time>,
    ) -> Json<RequiredTimes> {
        Json(RequiredTimes {
            timestamp: timestamp.0,
            zoned: zoned.0,
            datetime: datetime.0,
            date: date.0,
            time: time.0,
        })
    }

    #[oai(path = "/optional-query", method = "get")]
    async fn optional_query(
        &self,
        timestamp: Query<Option<Timestamp>>,
        zoned: Query<Option<Zoned>>,
        datetime: Query<Option<DateTime>>,
        date: Query<Option<Date>>,
        time: Query<Option<Time>>,
    ) -> Json<OptionalTimes> {
        Json(OptionalTimes {
            timestamp: timestamp.0,
            zoned: zoned.0,
            datetime: datetime.0,
            date: date.0,
            time: time.0,
        })
    }

    #[oai(path = "/multipart", method = "post")]
    async fn multipart(&self, values: RequiredTimes) -> Json<RequiredTimes> {
        Json(values)
    }

    #[oai(path = "/optional-multipart", method = "post")]
    async fn optional_multipart(&self, values: OptionalTimes) -> Json<OptionalTimes> {
        Json(values)
    }

    #[oai(path = "/json", method = "post")]
    async fn json(&self, values: Json<RequiredTimes>) -> Json<RequiredTimes> {
        values
    }
}

#[test]
fn query_parameter_schemas() {
    let meta = Api::meta();
    for (path, required) in [("/query", true), ("/optional-query", false)] {
        let operation = &meta[0]
            .paths
            .iter()
            .find(|item| item.path == path)
            .unwrap()
            .operations[0];
        assert_eq!(operation.params.len(), FIELDS.len());
        for (param, &(name, _, format)) in operation.params.iter().zip(&FIELDS) {
            assert_eq!(param.name, name);
            assert_eq!(param.required, required);
            let schema = param.schema.unwrap_inline();
            assert_eq!(schema.ty, "string");
            assert_eq!(schema.format, Some(format));
        }
    }
}

#[tokio::test]
async fn query_extraction() {
    let cli = TestClient::new(OpenApiService::new(Api, "jiff", "1.0"));
    for path in ["/query", "/optional-query"] {
        let mut request = cli.get(path);
        for (name, value, _) in FIELDS {
            request = request.query(name, &value);
        }
        let response = request.send().await;
        response.assert_status_is_ok();
        response.assert_json(output_json()).await;
    }

    let response = cli.get("/optional-query").send().await;
    response.assert_status_is_ok();
    response.assert_json(null_json()).await;
}

#[tokio::test]
async fn query_rejects_missing_and_invalid_fields() {
    let cli = TestClient::new(OpenApiService::new(Api, "jiff", "1.0"));
    for (target, _, _) in FIELDS {
        let mut request = cli.get("/query");
        for (name, value, _) in FIELDS {
            if name != target {
                request = request.query(name, &value);
            }
        }
        request.send().await.assert_status(StatusCode::BAD_REQUEST);

        for path in ["/query", "/optional-query"] {
            for invalid in ["not-a-date", ""] {
                let mut request = cli.get(path);
                for (name, value, _) in FIELDS {
                    request = request.query(name, &if name == target { invalid } else { value });
                }
                request.send().await.assert_status(StatusCode::BAD_REQUEST);
            }
        }
    }
}

#[tokio::test]
async fn multipart_extraction() {
    let cli = TestClient::new(OpenApiService::new(Api, "jiff", "1.0"));
    for path in ["/multipart", "/optional-multipart"] {
        let form = FIELDS
            .iter()
            .fold(TestForm::new(), |form, &(name, value, _)| {
                form.text(name, value)
            });
        let response = cli.post(path).multipart(form).send().await;
        response.assert_status_is_ok();
        response.assert_json(output_json()).await;
    }

    let response = cli
        .post("/optional-multipart")
        .multipart(TestForm::new())
        .send()
        .await;
    response.assert_status_is_ok();
    response.assert_json(null_json()).await;
}

#[tokio::test]
async fn multipart_rejects_missing_and_invalid_fields() {
    let cli = TestClient::new(OpenApiService::new(Api, "jiff", "1.0"));
    for (target, _, _) in FIELDS {
        let mut form = TestForm::new();
        for (name, value, _) in FIELDS {
            if name != target {
                form = form.text(name, value);
            }
        }
        let response = cli.post("/multipart").multipart(form).send().await;
        response.assert_status(StatusCode::BAD_REQUEST);
        response
            .assert_text(format!(
                "parse multipart error: field `{target}` is required"
            ))
            .await;

        for path in ["/multipart", "/optional-multipart"] {
            for invalid in ["not-a-date", ""] {
                let form = FIELDS
                    .iter()
                    .fold(TestForm::new(), |form, &(name, value, _)| {
                        form.text(name, if name == target { invalid } else { value })
                    });
                let response = cli.post(path).multipart(form).send().await;
                response.assert_status(StatusCode::BAD_REQUEST);
                let body = response.0.into_body().into_string().await.unwrap();
                assert!(
                    body.contains(&format!("failed to parse field `{target}`")),
                    "{body}"
                );
            }
        }
    }
}

#[tokio::test]
async fn json_payload_roundtrip() {
    let cli = TestClient::new(OpenApiService::new(Api, "jiff", "1.0"));
    let response = cli.post("/json").body_json(&input_json()).send().await;
    response.assert_status_is_ok();
    response.assert_json(output_json()).await;

    for (name, _, _) in FIELDS {
        let mut value = input_json();
        value[name] = json!("not-a-date");
        cli.post("/json")
            .body_json(&value)
            .send()
            .await
            .assert_status(StatusCode::BAD_REQUEST);
    }
}
