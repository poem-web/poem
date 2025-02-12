use poem::http::Method;
use poem_openapi::{
    param::{Path, Query},
    payload::Json,
    registry::{
        MetaExternalDocument, MetaMediaType, MetaOperationParam, MetaParamIn, MetaRequest,
        MetaResponse, MetaResponses,
    },
    types::Type,
    OpenApiService, Tags, Webhook,
};

#[tokio::test]
async fn name() {
    #[Webhook]
    #[allow(dead_code)]
    trait MyWebhooks {
        #[oai(name = "a", method = "post")]
        fn test1(&self);

        #[oai(method = "trace")]
        fn test2(&self);
    }

    assert_eq!(<&dyn MyWebhooks>::meta()[0].name, "a");
    assert_eq!(<&dyn MyWebhooks>::meta()[1].name, "test2");
}

#[tokio::test]
async fn method() {
    #[Webhook]
    #[allow(dead_code)]
    trait MyWebhooks {
        #[oai(method = "post")]
        fn test1(&self);

        #[oai(method = "trace")]
        fn test2(&self);
    }

    assert_eq!(<&dyn MyWebhooks>::meta()[0].operation.method, Method::POST);
    assert_eq!(<&dyn MyWebhooks>::meta()[1].operation.method, Method::TRACE);
}

#[tokio::test]
async fn deprecated() {
    #[Webhook]
    #[allow(dead_code)]
    trait MyWebhooks {
        #[oai(method = "post")]
        fn test1(&self);

        #[oai(method = "get", deprecated)]
        fn test2(&self);
    }

    assert!(!<&dyn MyWebhooks>::meta()[0].operation.deprecated);
    assert!(<&dyn MyWebhooks>::meta()[1].operation.deprecated);
}

#[tokio::test]
async fn tags() {
    #[derive(Tags)]
    enum MyTags {
        A,
        B,
        C,
    }

    #[Webhook(tag = "MyTags::A")]
    #[allow(dead_code)]
    trait MyWebhooks: Sync {
        #[oai(method = "post", tag = "MyTags::B", tag = "MyTags::C")]
        fn test1(&self);

        #[oai(method = "get", tag = "MyTags::B")]
        fn test2(&self);
    }

    assert_eq!(
        <&dyn MyWebhooks>::meta()[0].operation.tags,
        &["A", "B", "C"]
    );
    assert_eq!(<&dyn MyWebhooks>::meta()[1].operation.tags, &["A", "B"]);
}

#[tokio::test]
async fn operation_id() {
    #[Webhook]
    #[allow(dead_code)]
    trait MyWebhooks {
        #[oai(method = "post", operation_id = "a")]
        fn test1(&self);

        #[oai(method = "get", operation_id = "b")]
        fn test2(&self);
    }

    assert_eq!(
        <&dyn MyWebhooks>::meta()[0].operation.operation_id,
        Some("a")
    );
    assert_eq!(
        <&dyn MyWebhooks>::meta()[1].operation.operation_id,
        Some("b")
    );
}

#[tokio::test]
async fn parameters() {
    #[Webhook]
    #[allow(dead_code)]
    trait MyWebhooks {
        #[oai(method = "post")]
        fn test(&self, a: Query<i32>, b: Path<String>);
    }

    assert_eq!(
        <&dyn MyWebhooks>::meta()[0].operation.params,
        vec![
            MetaOperationParam {
                name: "a".to_string(),
                schema: i32::schema_ref(),
                in_type: MetaParamIn::Query,
                description: None,
                required: true,
                deprecated: false,
                explode: true,
                style: None,
            },
            MetaOperationParam {
                name: "b".to_string(),
                schema: String::schema_ref(),
                in_type: MetaParamIn::Path,
                description: None,
                required: true,
                deprecated: false,
                explode: true,
                style: None,
            }
        ]
    );
}

#[tokio::test]
async fn request_body() {
    #[Webhook]
    trait MyWebhooks {
        #[oai(method = "post")]
        #[allow(dead_code)]
        fn test(&self, req: Json<i32>);
    }

    assert_eq!(
        <&dyn MyWebhooks>::meta()[0].operation.request,
        Some(MetaRequest {
            description: None,
            content: vec![MetaMediaType {
                content_type: "application/json; charset=utf-8",
                schema: i32::schema_ref(),
            }],
            required: true
        })
    );
}

#[tokio::test]
async fn response() {
    #[Webhook]
    trait MyWebhooks {
        #[oai(method = "post")]
        #[allow(dead_code)]
        fn test(&self) -> Json<i32>;
    }

    assert_eq!(
        <&dyn MyWebhooks>::meta()[0].operation.responses,
        MetaResponses {
            responses: vec![MetaResponse {
                description: "",
                status: Some(200),
                status_range: None,
                content: vec![MetaMediaType {
                    content_type: "application/json; charset=utf-8",
                    schema: i32::schema_ref(),
                }],
                headers: vec![]
            }]
        }
    );
}

#[tokio::test]
async fn create() {
    #[Webhook]
    trait MyWebhooks {
        #[oai(method = "post")]
        #[allow(dead_code)]
        fn test(&self) -> Json<i32>;
    }

    let _ = OpenApiService::new((), "Test", "1.0").webhooks::<&dyn MyWebhooks>();
}

#[tokio::test]
async fn external_docs() {
    #[Webhook]
    trait MyWebhooks {
        #[oai(
            method = "post",
            external_docs = "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
        )]
        #[allow(dead_code)]
        fn test(&self);
    }

    assert_eq!(
        <&dyn MyWebhooks>::meta()[0].operation.external_docs,
        Some(MetaExternalDocument {
            url: "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
                .to_string(),
            description: None
        })
    );
}
