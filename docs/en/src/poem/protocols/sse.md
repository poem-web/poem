# Server-Sent Events (SSE)

SSE allows the server to continuously push data to the client.

You need to create a `SSE` response with a type that implements `Stream<Item=Event>`.

The endpoint in the example below will send three events.

```rust
use futures_util::stream;
use poem::{
    handler, Route, get,
    http::StatusCode,
    web::sse::{Event, SSE},
    Endpoint, Request,
};

#[handler]
fn index() -> SSE {
    SSE::new(stream::iter(vec![
        Event::message("a"),
        Event::message("b"),
        Event::message("c"),
    ]))
}

let app = Route::new().at("/", get(index));
```
