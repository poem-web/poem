Define a OpenAPI.

# Macro parameters

| Attribute     | description               | Type     | Optional |
|---------------|---------------------------|----------|----------|
| prefix_path   | Define the prefix for all operation paths  | string | Y |
| tag           | Define a tag for all operations. | string   | Y        |

# Operation parameters

| Attribute     | description               | Type     | Optional |
|---------------|---------------------------|----------|----------|
| path          | HTTP uri.                 | string   | N        |
| method        | HTTP method. The possible values are "get", "post", "put", "delete", "head", "options", "connect", "patch", "trace". | string   | N        |
| deprecated    | Operation deprecated      | bool     | Y        |
| tag           | Operation tag             | Tags     | Y        |
| operation_id  | Unique string used to identify the operation. | string | Y |
| transform     | Use a function to transform the API endpoint. | string | Y |

# Operation argument parameters

| Attribute     | description               | Type     | Optional |
|---------------|---------------------------|----------|----------|
| name          | Parameter name | string   | Y        |
| desc          | Argument description      | string   | Y        |
| deprecated    | Argument deprecated       | bool     | Y        |
| default       | Default value            | bool,string | Y     |
| validator.multiple_of   | The value of "multiple_of" MUST be a number, strictly greater than 0. A numeric instance is only valid if division by this value results in an integer. | number | Y |
| validator.maximum       | The value of "maximum" MUST be a number, representing an upper limit for a numeric instance. If `exclusive` is `true` and instance is less than the provided value, or else if the instance is less than or exactly equal to the provided value. | { value: `<number>`, exclusive: `<bool>`} | Y |
| validator.minimum       | The value of "minimum" MUST be a number, representing a lower limit for a numeric instance. If `exclusive` is `true` and instance is greater than the provided value, or else if the instance is greater than or exactly equal to the provided value. | { value: `<number>`, exclusive: `<bool>`} | Y |
| validator.max_length    | The value of "max_length" MUST be a non-negative integer. A string instance is valid against this validator if its length is less than, or equal to, the value. | usize | Y |
| validator.min_length    | The value of "min_length" MUST be a non-negative integer.  The value of this validator MUST be an integer. This integer MUST be greater than, or equal to, 0.| usize | Y |
| validator.pattern       | The value of "pattern" MUST be a string. This string SHOULD be a valid regular expression, according to the ECMA 262 regular expression dialect. A string instance is considered valid if the regular expression matches the instance successfully. | string | Y |
| validator.max_items     | The value of "max_items" MUST be an integer. This integer MUST be greater than, or equal to, 0. An array instance is valid if its size is less than, or equal to, the value of this validator. | usize | Y |
| validator.min_items     | The value of "min_items" MUST be an integer. This integer MUST be greater than, or equal to, 0. An array instance is valid if its size is greater than, or equal to, the value of this validator. | usize | Y |
| validator.unique_items  | The value of "unique_items" MUST be an boolean.  If this value is `false`, the instance validates successfully.  If this value is `true`, the instance validates successfully if all of its elements are unique. | bool | Y |
| validator.max_properties | The value of this keyword MUST be a non-negative integer. An object instance is valid against "maxProperties" if its number of properties is less than, or equal to, the value of this keyword. | usize | Y |
| validator.min_properties | The value of this keyword MUST be a non-negative integer. An object instance is valid against "minProperties" if its number of properties is greater than, or equal to, the value of this keyword. | usize | Y |

# Examples

```rust
use poem_openapi::{
    param::Header,
    payload::{Json, PlainText},
    ApiRequest, Object, OpenApi, ApiResponse,
};

#[derive(Object)]
struct Pet {
    id: String,
    name: String,
}

#[derive(ApiRequest)]
enum CreatePetRequest {
    /// This request receives a pet in JSON format(application/json).
    CreateByJSON(Json<Pet>),
    /// This request receives a pet in text format(text/plain).
    CreateByPlainText(PlainText<String>),
}

#[derive(ApiResponse)]
enum CreatePetResponse {
    /// Returns when the pet is successfully created.
    #[oai(status = 200)]
    Ok,
    /// Returns when the pet already exists.
    #[oai(status = 409)]
    PetAlreadyExists,
}

struct PetApi;

#[OpenApi]
impl PetApi {
    /// Create a new pet.
    #[oai(path = "/pet", method = "post")]
    async fn create_pet(
        &self,
        #[oai(name = "TOKEN")] token: Header<String>,
        req: CreatePetRequest
    ) -> CreatePetResponse {
        todo!()
    }
}
```