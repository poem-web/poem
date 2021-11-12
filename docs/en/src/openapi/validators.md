# Validators

The `OpenAPI` specification supports validation based on `Json Schema`, and `Poem-openapi` also supports them. You can 
apply validators to operation parameters, object members, and `Multipart` fields. The validator can only work on specific 
data types, otherwise it will fail to compile. For example, `maximum` can only be used for numeric types, and `max_items` 
can only be used for array types.

For more validators, please refer to [document](https://docs.rs/poem-openapi/0.8.0/poem_openapi/attr.OpenApi.html#operation-argument-parameters).

```rust
use poem_openapi::{Object, OpenApi, Multipart};

#[derive(Object)]
struct Pet {
    id: u64,

    /// The length of the name must be less than 32
    #[oai(max_length = "32")]
    name: String,

    /// Array length must be less than 3
    #[oai(max_items = "3")]
    photo_urls: Vec<String>,

    status: PetStatus,
}
```
