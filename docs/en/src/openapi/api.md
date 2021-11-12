# API

The following defines some API operations to add, delete, modify and query the `pet` table.

`add_pet` and `update_pet` are used to add and update the `Pet` object. **This is the basic type we defined before. 
The basic type cannot be directly used as the request content. You need to use a `Payload` type to wrap it**, In this way,
you can determine what the requested `Content-Type` is. In the following example, we use `payload::Json` to wrap it, 
indicating that the `Content-Type` of these two API requests is `application/json`.

`find_pet_by_id` and `find_pets_by_status` are used to find the `Pet` object, and their response is also a `Pet` object, 
which also needs to be wrapped with the `Payload` type.

We can use `#[oai(name = "...", in = "...")]` to decorate a function parameter to specify the source of this value. 
The `in` attribute can be `query`, ` path`, `header` and `cookie`. The `id` parameter of `delete_pet` is parsed from the 
path, and the parameters of `find_pet_by_id` and `find_pets_by_status` are parsed from the Url query string. If the 
parameter type is not `Option<T>`, it means that this parameter is not an optional parameter, and a `400 Bad Request` error 
will be returned when the parsing fails.

You can define multiple function parameters, but there can only be one `Payload` type as the request content, or multiple 
basic types as the request parameters.

```rust
use poem_api::{
  OpenApi,
  poem_api::payload::Json,
};
use poem::Result;

struct Api;

#[OpenApi]
impl Api {
    /// Add new pet
    #[oai(path = "/pet", method = "post")]
    async fn add_pet(&self, pet: Json<Pet>) -> Result<()> {
        todo!()
    }
  
    /// Update existing pet
    #[oai(path = "/pet", method = "put")]
    async fn update_pet(&self, pet: Json<Pet>) -> Result<()> {
        todo!()
    }

    /// Delete a pet
    #[oai(path = "/pet/:pet_id", method = "delete")]
    async fn delete_pet(&self, #[oai(name = "pet_id", in = "path")] id: u64) -> Result<()> {
        todo!()
    }
  
    /// Query pet by id
    #[oai(path = "/pet", method = "get")]
    async fn find_pet_by_id(&self, #[oai(name = "status", in = "query")] id: u64) -> Result<Json<Pet>> {
        todo!()
    } 
  
    /// Query pets by status
    #[oai(path = "/pet/findByStatus", method = "get")]
    async fn find_pets_by_status(&self, #[oai(name = "status", in = "query")] status: Status) -> Result<Json<Vec<Pet>>> {
        todo!()
    }
}
```
