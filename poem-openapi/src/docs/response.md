Define a OpenAPI response.

# Macro parameters

| Attribute           | description                                                                                     | Type   | Optional |
|---------------------|-------------------------------------------------------------------------------------------------|--------|----------|
| bad_request_handler | Sets a custom bad request handler, it can convert error to the value of the this response type. | string | Y        |

# Item parameters

| Attribute | description                                                  | Type | Optional |
|-----------|--------------------------------------------------------------|------|----------|
| status    | HTTP status code. If omitted, it is a default response type. | u16  | Y        |

# Header parameters

| Attribute   | description               | Type     | Optional |
|-------------|---------------------------|----------|----------|
| name        | Header name               | String   | Y        |
| desc        | Header description        | String   | Y        |

# Examples

```rust
use poem::Error;
use poem_openapi::{payload::PlainText, ApiResponse};

#[derive(ApiResponse)]
#[oai(bad_request_handler = "bad_request_handler")]
enum CreateUserResponse {
    /// Returns when the user is successfully created.
    #[oai(status = 200)]
    Ok,
    /// Returns when the user already exists.
    #[oai(status = 409)]
    UserAlreadyExists,
    /// Returns when the request parameters is incorrect.
    #[oai(status = 400)]
    BadRequest(PlainText<String>),
}

// Convert error to `CreateUserResponse::BadRequest`.
fn bad_request_handler(err: Error) -> CreateUserResponse {
    CreateUserResponse::BadRequest(PlainText(format!("error: {}", err.to_string())))
}
```