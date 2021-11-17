# Responses

All types that can be converted to HTTP response `Response` should implement `IntoResponse`, and they can be used as the 
return value of the handler function.

In the following example, the `string_response` and `status_response` functions return the `String` and `StatusCode` 
types, because `Poem` has implemented the `IntoResponse` feature for them.

The `no_response` function does not return a value. We can also think that its return type is `()`, and `Poem` also 
implements `IntoResponse` for `()`, which is always converted to `200 OK`.

```rust
use poem::handler;
use poem::http::StatusCode;

#[handler]
fn string_response() -> String {
    "hello".to_string()
}

#[handler]
fn status_response() -> StatusCode {}

#[handler]
fn no_response() {}

```

# Built-in responses

- **Result&lt;T: IntoResponse, E: IntoResponse>**

    if the result is `Ok`, use the `Ok` value as the response, otherwise use the `Err` value.

- **()**

   Sets the status to `OK` with an empty body.

- **&'static str**

   Sets the status to `OK` and the `Content-Type` to `text/plain`. The
string is used as the body of the response.

- **String**

   Sets the status to `OK` and the `Content-Type` to `text/plain`. The
string is used as the body of the response.

- **&'static [u8]**

   Sets the status to `OK` and the `Content-Type` to
`application/octet-stream`. The slice is used as the body of the response.

- **Html&lt;T>**

   Sets the status to `OK` and the `Content-Type` to `text/html`. `T` is
used as the body of the response.

- **Json&lt;T>**

   Sets the status to `OK` and the `Content-Type` to `application/json`. Use
[`serde_json`](https://crates.io/crates/serde_json) to serialize `T` into a json string.

- **Bytes**

   Sets the status to `OK` and the `Content-Type` to
`application/octet-stream`. The bytes is used as the body of the response.

- **Vec&lt;u8>**

   Sets the status to `OK` and the `Content-Type` to
`application/octet-stream`. The vector’s data is used as the body of the
response.

- **Body**

  Sets the status to `OK` and use the specified body.

- **StatusCode**

   Sets the status to the specified status code `StatusCode` with an empty
body.

- **(StatusCode, T)**

   Convert `T` to response and set the specified status code `StatusCode`.

- **(StatusCode, HeaderMap, T)**

   Convert `T` to response and set the specified status code `StatusCode`,
and then merge the specified `HeaderMap`.

- **Response**

   The implementation for `Response` always returns itself.

- **Compress&lt;T>**

   Call `T::into_response` to get the response, then compress the response
body with the specified algorithm, and set the correct `Content-Encoding`
header.

- **SSE**

    Sets the status to `OK` and the `Content-Type` to `text/event-stream`
with an event stream body. Use the `SSE::new` function to
create it.

## Custom response

In the following example, we wrap a response called `PDF`, which adds a `Content-Type: applicationn/pdf` header to the response.

```rust
use poem::{IntoResponse, Response};

struct PDF(Vec<u8>);

impl IntoResponse for PDF {
    fn into_response(self) -> Response { 
        Response::builder()
            .header("Content-Type", "application/pdf")
            .body(self.0)
    }
}
```
