# Custom Response

In all the previous examples, all operations return `Result`. When an error occurs, a `poem::Error` is returned, which 
contains the reason and status code of the error. However, the `OpenAPI` specification supports a more detailed definition
of the response of the operation, such as which status codes may be returned, and the reason for the status code and the
content of the response.

In the following example, we change the return type of the `create_post` function to `CreateBlogResponse`.

`Ok`, `Forbidden` and `InternalError` specify the response content of a specific status code.

```rust
use poem_openapi::ApiResponse;
use poem::http::StatusCode;

#[derive(ApiResponse)]
enum CreateBlogResponse {
    /// Created successfully
    #[oai(status = 200)]
    Ok(Json<u64>),
    
    /// Permission denied
    #[oai(status = 403)]
    Forbidden,
  
    /// Internal error
    #[oai(status = 500)]
    InternalError,
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn create_post(
        &self,
        req: CreatePostRequest,
    ) -> CreateBlogResponse {
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

When the parsing request fails, the default `400 Bad Request` error will be returned, but sometimes we want to return a 
custom error content, we can use the `bad_request_handler` attribute to set an error handling function, this function is
used to convert `ParseRequestError` to specified response type.

```rust
use poem_openapi::{
    ApiResponse, Object, ParseRequestError, payload::Json,
};

#[derive(Object)]
struct ErrorMessage {
    code: i32,
    reason: String,
}

#[derive(ApiResponse)]
#[oai(bad_request_handler = "bad_request_handler")]
enum CreateBlogResponse {
    /// Created successfully
    #[oai(status = 200)]
    Ok(Json<u64>),

    /// Permission denied
    #[oai(status = 403)]
    Forbidden,

    /// Internal error
    #[oai(status = 500)]
    InternalError,
    
    /// Bad request
    #[oai(status = 400)]
    BadRequest(Json<ErrorMessage>),
}

fn bad_request_handler(err: ParseRequestError) -> CreateBlogResponse {
    // When the parsing request fails, a custom error content is returned, which is a JSON
    CreateBlogResponse::BadRequest(Json(ErrorMessage {
        code: -1,
        reason: err.to_string(),
    }))
}
```
