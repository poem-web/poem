Define a OpenAPI request.

# Item parameters

| Attribute    | Description               | Type   | Optional |
|--------------|---------------------------|--------|----------|
| content_type | Specify the content type. | string | Y        |

# Examples

```rust
use poem_openapi::{
    payload::{Json, PlainText},
    ApiRequest, Object,
};

#[derive(Object)]
struct Pet {
    id: String,
    name: String,
}

#[derive(ApiRequest)]
enum CreatePet {
    /// This request receives a pet in JSON format(application/json).
    CreateByJSON(Json<Pet>),
    /// This request receives a pet in text format(text/plain).
    CreateByPlainText(PlainText<String>),
}
```