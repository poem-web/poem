# Upload files

The `Multipart` macro is usually used for file upload. It can define a form to contain one or more files and some 
additional fields. The following example provides an operation to create a `Pet` object, which can upload some image 
files at the same time.

```rust
use poem_openapi::{Multipart, OpenApi};
use poem::Result;

#[derive(Debug, Multipart)]
struct CreatePetPayload {
    name: String,
    status: PetStatus,
    photos: Vec<Upload>, // some photos
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/pet", method = "post")]
    async fn create_pet(&self, payload: CreatePetPayload) -> Result<Json<u64>> {
        todo!()
    }
}
```

For the complete example, please refer to [Upload Example](https://github.com/poem-web/poem/tree/master/examples/openapi/upload).
