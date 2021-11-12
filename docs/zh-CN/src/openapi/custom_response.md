# 自定义响应

在前面的例子中，我们的所有请求处理函数都返回的`Result`类型，当发生错误时返回一个`poem::Error`，它包含错误的原因以及状态码。但`OpenAPI`规范允许更详细的描述请求的响应，例如该接口可能会返回哪些状态码，以及状态码对应的原因和响应的内容。

在下面的例子中，我们修改`create_post`函数的返回值为`CreateBlogResponse`类型。

`Ok`，`Forbidden`和`InternalError`描述了特定状态码的响应类型。

```rust
use poem_openapi::ApiResponse;
use poem::http::StatusCode;

#[derive(ApiResponse)]
enum CreateBlogResponse {
    /// 创建完成
    #[oai(status = 200)]
    Ok(Json<u64>),
    
    /// 没有权限
    #[oai(status = 403)]
    Forbidden,
  
    /// 内部错误
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

当请求解析失败时，默认会返回`400 Bad Request`错误，但有时候我们想返回一个自定义的错误内容，可以使用`bad_request_handler`属性设置一个错误处理函数，这个函数用于转换`ParseRequestError`到指定的响应类型。

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
    /// 创建完成
    #[oai(status = 200)]
    Ok(Json<u64>),

    /// 没有权限
    #[oai(status = 403)]
    Forbidden,

    /// 内部错误
    #[oai(status = 500)]
    InternalError,
    
    /// 请求无效
    #[oai(status = 400)]
    BadRequest(Json<ErrorMessage>),
}

fn bad_request_handler(err: ParseRequestError) -> CreateBlogResponse {
    // 当解析请求失败时，返回一个自定义的错误内容，它是一个JSON
    CreateBlogResponse::BadRequest(Json(ErrorMessage {
        code: -1,
        reason: err.to_string(),
    }))
}
```
