# 自定义请求

`OpenAPI`规范允许同一个接口支持处理不同`Content-Type`的请求，例如一个接口可以同时接受`application/json`和`text/plain`类型的Payload。

在`Poem-openapi`中，要支持此类型请求，需要用`ApiRequest`宏自定义一个实现了`Payload trait`的请求对象。

在下面的例子中，`create_post`函数接受`CreatePostRequest`请求，当创建成功后，返回`id`。

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
    /// 从JSON创建
    Json(Json<Blog>),
    /// 从文本创建
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
