Define a OpenAPI request.

# Examples

```rust
use poem_openapi::{
    payload::{Json, PlainText},
    Request, Object,
};

#[derive(Object)]
struct Pet {
    id: String,
    name: String,
}

#[derive(Request)]
enum CreatePet {
    /// This request receives a pet in JSON format(application/json).
    CreateByJSON(Json<Pet>),
    /// This request receives a pet in text format(text/plain).
    CreateByPlainText(PlainText),
}
```