use poem::{Error, http::StatusCode, test::TestClient};
use poem_openapi::{
    ApiResponse, OpenApi, OpenApiService,
    param::Query,
    payload::{Json, Response},
};

#[tokio::test]
async fn response_wrapper() {
    #[derive(ApiResponse, Debug, Eq, PartialEq)]
    #[oai(bad_request_handler = "bad_request_handler")]
    #[allow(dead_code)]
    pub enum CustomApiResponse {
        #[oai(status = 200)]
        Ok,
        #[oai(status = 400)]
        BadRequest(#[oai(header = "MY-HEADER1")] String),
    }

    fn bad_request_handler(_: Error) -> CustomApiResponse {
        CustomApiResponse::BadRequest("def".to_string())
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/a", method = "get")]
        async fn a(&self) -> Response<Json<i32>> {
            Response::new(Json(100)).header("myheader", "abc")
        }

        #[oai(path = "/b", method = "get")]
        async fn b(&self, p1: Query<String>) -> Response<CustomApiResponse> {
            Response::new(CustomApiResponse::Ok).header("myheader", p1.0)
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.get("/a").send().await;
    resp.assert_status_is_ok();
    resp.assert_header("myheader", "abc");

    let resp = cli.get("/b").query("p1", &"qwe").send().await;
    resp.assert_status_is_ok();
    resp.assert_header("myheader", "qwe");

    let resp = cli.get("/b").send().await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    resp.assert_header("MY-HEADER1", "def");
}

#[test]
fn payload_response_metadata() {
    use poem_openapi::{
        payload::{Base64, Binary, EventStream, Html, Payload, PlainText, Xml, Yaml},
        registry::{MetaMediaType, MetaResponse, MetaResponses},
    };

    #[derive(poem_openapi::Object)]
    struct TestPayload {
        value: i32,
    }

    fn check<T: ApiResponse + Payload>(content_type: &'static str) {
        assert_eq!(
            T::meta(),
            MetaResponses {
                responses: vec![MetaResponse {
                    description: "",
                    status: Some(200),
                    status_range: None,
                    content: vec![MetaMediaType {
                        content_type,
                        schema: T::schema_ref(),
                    }],
                    headers: vec![],
                }],
            }
        );
    }

    check::<Json<i32>>("application/json; charset=utf-8");
    check::<Xml<i32>>("application/xml; charset=utf-8");
    check::<Yaml<TestPayload>>("application/yaml; charset=utf-8");
    check::<PlainText<String>>("text/plain; charset=utf-8");
    check::<Html<String>>("text/html; charset=utf-8");
    check::<Binary<Vec<u8>>>("application/octet-stream");
    check::<Base64<Vec<u8>>>("text/plain; charset=utf-8");
    check::<EventStream<futures_util::stream::Empty<i32>>>("text/event-stream");
}

#[derive(poem_openapi::Object)]
struct TextPayload {
    text: String,
}

#[tokio::test]
async fn request_payloads_preserve_fragmented_unicode() {
    use bytes::Bytes;
    use poem::{Body, Request, RequestBody};
    use poem_openapi::payload::{Form, ParsePayload, Xml, Yaml};
    use serde_json::{Value, json};

    fn fragmented(parts: &'static [&'static [u8]]) -> RequestBody {
        RequestBody::new(Body::from_bytes_stream(futures_util::stream::iter(
            parts
                .iter()
                .map(|part| Ok::<_, std::io::Error>(Bytes::from_static(part))),
        )))
    }

    let req = Request::default();
    let Json(value) = Json::<Value>::from_request(
        &req,
        &mut fragmented(&[b"{\"text\":\"\xe4", b"\xbd\xa0\"}"]),
    )
    .await
    .unwrap();
    assert_eq!(value, json!({ "text": "你" }));

    let Xml(value) = Xml::<Value>::from_request(
        &req,
        &mut fragmented(&[b"<root><text>\xe4", b"\xbd\xa0</text></root>"]),
    )
    .await
    .unwrap();
    assert_eq!(value, json!({ "text": { "$text": "你" } }));

    let Yaml(value) =
        Yaml::<TextPayload>::from_request(&req, &mut fragmented(&[b"text: \xe4", b"\xbd\xa0\n"]))
            .await
            .unwrap();
    assert_eq!(value.text, "你");

    #[derive(serde::Deserialize)]
    struct FormData {
        text: String,
    }
    let Form(value) =
        Form::<FormData>::from_request(&req, &mut fragmented(&[b"text=%E4%", b"BD%A0"]))
            .await
            .unwrap();
    assert_eq!(value.text, "你");
}

#[tokio::test]
async fn request_payloads_preserve_empty_and_error_behavior() {
    use bytes::Bytes;
    use poem::{Body, Request, RequestBody, error::ReadBodyError};
    use poem_openapi::{
        error::ParseRequestPayloadError,
        payload::{Form, ParsePayload, Xml, Yaml},
    };
    use serde_json::Value;

    async fn errors<P: ParsePayload>(invalid: &'static [u8]) {
        let req = Request::default();
        let mut body = RequestBody::new(Body::from(invalid));
        let err = P::from_request(&req, &mut body).await.err().unwrap();
        assert!(err.is::<ParseRequestPayloadError>());
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);

        let err = P::from_request(&req, &mut body).await.err().unwrap();
        assert!(matches!(
            err.downcast_ref::<ReadBodyError>(),
            Some(ReadBodyError::BodyHasBeenTaken)
        ));

        let stream = futures_util::stream::once(async {
            Err::<Bytes, _>(std::io::Error::other("read failed"))
        });
        let mut body = RequestBody::new(Body::from_bytes_stream(stream));
        let err = P::from_request(&req, &mut body).await.err().unwrap();
        assert!(matches!(
            err.downcast_ref::<ReadBodyError>(),
            Some(ReadBodyError::Io(_))
        ));
    }

    errors::<Json<Value>>(b"{").await;
    errors::<Json<Value>>(b"\"\xff\"").await;
    errors::<Xml<Value>>(b"<root>").await;
    errors::<Yaml<TextPayload>>(b"[invalid").await;
    errors::<Yaml<TextPayload>>(b"text: \xff").await;
    errors::<Form<std::collections::BTreeMap<String, u32>>>(b"number=invalid").await;

    let req = Request::default();
    let mut body = RequestBody::new(Body::empty());
    assert!(
        Json::<Option<Value>>::from_request(&req, &mut body)
            .await
            .unwrap()
            .0
            .is_none()
    );
    let mut body = RequestBody::new(Body::empty());
    assert_eq!(
        Xml::<Value>::from_request(&req, &mut body).await.unwrap().0,
        Value::Null
    );
    let mut body = RequestBody::new(Body::empty());
    let err = Yaml::<TextPayload>::from_request(&req, &mut body)
        .await
        .err()
        .unwrap();
    let expected =
        <TextPayload as poem_openapi::types::ParseFromYAML>::parse_from_yaml(Some(Value::Null))
            .err()
            .unwrap()
            .into_message();
    assert_eq!(
        err.downcast_ref::<ParseRequestPayloadError>()
            .unwrap()
            .reason,
        expected
    );

    // Invalid UTF-8 has historically been parsed as an empty XML string.
    let mut body = RequestBody::new(Body::from(b"<root>\xff</root>".as_slice()));
    let err = Xml::<Value>::from_request(&req, &mut body)
        .await
        .err()
        .unwrap();
    let expected = quick_xml::de::from_str::<Value>("")
        .unwrap_err()
        .to_string();
    assert_eq!(
        err.downcast_ref::<ParseRequestPayloadError>()
            .unwrap()
            .reason,
        expected
    );
}
