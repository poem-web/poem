# Custom Request

The `OpenAPI` specification allows the same operation to support processing different requests of `Content-Type`, 
for example, an operation can support `application/json` and `text/plain` types of request content.

In `Poem-openapi`, to support this type of request, you need to use the `ApiRequest` macro to customize a request object 
that implements the `Payload` trait.

In the following example, the `create_post` function accepts the `CreatePostRequest` request, and when the creation is 
successful, it returns the `id`.

```rust
use poem_open::{
    ApiRequest, Object,
    payload::{PlainText, Json},
};
use poem::Result;

#[derive(Object)]
struct Post {
    title: String,
    content: String,
}

#[derive(ApiRequest)]
enum CreatePostRequest {
    /// Create from json
    Json(Json<Blog>),
    /// Create from plain text
    Text(PlainText<String>),
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "post")]
    async fn create_post(
        &self,
        req: CreatePostRequest,
    ) -> Result<Json<u64>> {
        match req {
            CreatePostRequest::Json(Json(blog)) => {
                todo!();
            }
            CreatePostRequest::Text(content) => {
                todo!();
            }
        }
    }
}
```
