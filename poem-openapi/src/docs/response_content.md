Define a OpenAPI response content.

# Item parameters

| Attribute    | Description                        | Type   | Optional |
|--------------|------------------------------------|--------|----------|
| content_type | Specify the content type.          | string | Y        |
| actual_type  | Specifies the actual response type | Y      | string   |

# Examples

```rust
use poem_openapi::{
    payload::{Binary, Json, PlainText},
    ApiResponse, ResponseContent,
};

#[derive(ResponseContent)]
enum MyResponseContent {
    A(Json<i32>),
    B(PlainText<String>),
    C(Binary<Vec<u8>>),
}

#[derive(ApiResponse)]
enum MyResponse {
    #[oai(status = 200)]
    Ok(MyResponseContent),
}
```
