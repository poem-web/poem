Define an OpenAPI response.

# Macro parameters

| Attribute           | Description                                                                                     | Type                                                       | Optional |
|---------------------|-------------------------------------------------------------------------------------------------|------------------------------------------------------------|----------|
| bad_request_handler | Sets a custom bad request handler, it can convert error to the value of the this response type. | string                                                     | Y        |
| header              | Add an extra header                                                                             | [`ExtraHeader`](macro@ApiResponse#extra-header-parameters) | Y        |
| display             | When converting a response to an error, the error message comes from the `Display trait`.       | bool                                                       | Y        |

# Item parameters

| Attribute    | description                                                  | Type                                                       | Optional |
|--------------|--------------------------------------------------------------|------------------------------------------------------------|----------|
| status       | HTTP status code. If omitted, it is a default response type. | u16                                                        | Y        |
| status_range | Specify a range of HTTP status codes.                        | string                                                     | Y        |
| content_type | Specify the content type.                                    | string                                                     | Y        |
| actual_type  | Specifies the actual response type                           | string                                                     | Y        |
| header       | Add an extra header                                          | [`ExtraHeader`](macro@ApiResponse#extra-header-parameters) | Y        |

# Header parameters

| Attribute  | description       | Type   | Optional |
|------------|-------------------|--------|----------|
| deprecated | Header deprecated | String | Y        |

# Extra header parameters

| Attribute   | description        | Type   | Optional |
|-------------|--------------------|--------|----------|
| name        | Header name        | String | N        |
| ty          | Header type        | String | N        |
| description | Header description | String | Y        |
| deprecated  | Header deprecated  | bool   | Y        |

# Example response headers

```rust
use poem_openapi::{payload::PlainText, ApiResponse};

#[derive(ApiResponse)]
enum CreateUserResponse {
    #[oai(status = 200)]
    Ok(#[oai(header = "X-Id")] String),
    #[oai(status = 201)]
    OkWithBody(PlainText<String>, #[oai(header = "X-Id")] String),
}
```

# Example extra headers

```rust
use poem_openapi::ApiResponse;

#[derive(ApiResponse)]
#[oai(
    header(name = "X-ExtraHeader-1", ty = "String"),
    header(name = "X-ExtraHeader-2", ty = "i32"),
)]
enum CreateUserResponse {
    #[oai(status = 200, header(name = "X-ExtraHeader-3", ty = "f32"))]
    Ok,
}
```

# Example status range

```rust
use poem::http::StatusCode;
use poem_openapi::{payload::PlainText, ApiResponse};

#[derive(ApiResponse)]
enum CreateUserResponse {
    #[oai(status_range = "2XX")]
    Ok(StatusCode, PlainText<String>),
    #[oai(status_range = "4XX")]
    ClientError(StatusCode),
    #[oai(status_range = "5XX")]
    ServerError(StatusCode),
}
```

# Example with bad request handler

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

# Example as an error type

```rust
use poem::Error;
use poem_openapi::{payload::{PlainText, Json}, ApiResponse, Object, OpenApi};

#[derive(Object)]
struct CreateUserRequest {
    username: String,
    nickname: String,
    email: String,
}

#[derive(ApiResponse)]
enum CreateUserResponse {
    /// Returns when the user is successfully created.
    #[oai(status = 200)]
    Ok,
}

#[derive(ApiResponse)]
enum CreateUserResponseError {
    /// Returns when the user already exists.
    #[oai(status = 409)]
    UserAlreadyExists,
    /// Returns when the request parameters is incorrect.
    #[oai(status = 400)]
    BadRequest(PlainText<String>),
}

struct UserApi;

#[OpenApi]
impl UserApi {
    /// Create a new user.
    #[oai(path = "/user", method = "post")]
    async fn create(
        &self,
        user: Json<CreateUserRequest>,
    ) -> Result<CreateUserResponse, CreateUserResponseError> {
        todo!()
    }
}
```
