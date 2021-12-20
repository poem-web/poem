use poem::http::Method;
use poem_openapi::{
    param::{Path, Query},
    payload::Json,
    registry::{
        MetaMediaType, MetaOperationParam, MetaParamIn, MetaRequest, MetaResponse, MetaResponses,
    },
    types::Type,
    OpenApiService, Tags, Webhook,
};

#[tokio::test]
async fn name() {
    #[Webhook]
    trait MyWebhooks: Sync {
        #[oai(name = "a", method = "post")]
        async fn test1(&self);

        #[oai(method = "trace")]
        async fn test2(&self);
    }

    assert_eq!(<&dyn MyWebhooks>::meta()[0].name, "a");
    assert_eq!(<&dyn MyWebhooks>::meta()[1].name, "test2");
}

#[tokio::test]
async fn method() {
    #[Webhook]
    trait MyWebhooks: Sync {
        #[oai(method = "post")]
        async fn test1(&self);

        #[oai(method = "trace")]
        async fn test2(&self);
    }

    assert_eq!(<&dyn MyWebhooks>::meta()[0].operation.method, Method::POST);
    assert_eq!(<&dyn MyWebhooks>::meta()[1].operation.method, Method::TRACE);
}

#[tokio::test]
async fn deprecated() {
    #[Webhook]
    trait MyWebhooks: Sync {
        #[oai(method = "post")]
        async fn test1(&self);

        #[oai(method = "get", deprecated)]
        async fn test2(&self);
    }

    assert_eq!(<&dyn MyWebhooks>::meta()[0].operation.deprecated, false);
    assert_eq!(<&dyn MyWebhooks>::meta()[1].operation.deprecated, true);
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
    trait MyWebhooks: Sync {
        #[oai(method = "post", tag = "MyTags::B", tag = "MyTags::C")]
        async fn test1(&self);

        #[oai(method = "get", tag = "MyTags::B")]
        async fn test2(&self);
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
    trait MyWebhooks: Sync {
        #[oai(method = "post", operation_id = "a")]
        async fn test1(&self);

        #[oai(method = "get", operation_id = "b")]
        async fn test2(&self);
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
    trait MyWebhooks: Sync {
        #[oai(method = "post")]
        async fn test(&self, a: Query<i32>, b: Path<String>);
    }

    assert_eq!(
        <&dyn MyWebhooks>::meta()[0].operation.params,
        vec![
            MetaOperationParam {
                name: "a",
                schema: i32::schema_ref(),
                in_type: MetaParamIn::Query,
                description: None,
                required: true,
                deprecated: false
            },
            MetaOperationParam {
                name: "b",
                schema: String::schema_ref(),
                in_type: MetaParamIn::Path,
                description: None,
                required: true,
                deprecated: false
            }
        ]
    );
}

#[tokio::test]
async fn request_body() {
    #[Webhook]
    trait MyWebhooks: Sync {
        #[oai(method = "post")]
        async fn test(&self, req: Json<i32>);
    }

    assert_eq!(
        <&dyn MyWebhooks>::meta()[0].operation.request,
        Some(MetaRequest {
            description: None,
            content: vec![MetaMediaType {
                content_type: "application/json",
                schema: i32::schema_ref(),
            }],
            required: true
        })
    );
}

#[tokio::test]
async fn response() {
    #[Webhook]
    trait MyWebhooks: Sync {
        #[oai(method = "post")]
        async fn test(&self) -> Json<i32>;
    }

    assert_eq!(
        <&dyn MyWebhooks>::meta()[0].operation.responses,
        MetaResponses {
            responses: vec![MetaResponse {
                description: "",
                status: Some(200),
                content: vec![MetaMediaType {
                    content_type: "application/json",
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
    trait MyWebhooks: Sync {
        #[oai(method = "post")]
        async fn test(&self) -> Json<i32>;
    }

    let _ = OpenApiService::new((), "Test", "1.0").webhooks::<dyn MyWebhooks>();
}
