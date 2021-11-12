# Endpoint

The endpoint can handle HTTP requests. You can implement the `Endpoint` trait to create your own endpoint.
`Poem` also provides some convenient functions to easily create a custom endpoint type.

In the previous chapter, we learned how to use the `handler` macro to convert a function to an endpoint.

Now let's see how to create your own endpoint by implementing the `Endpoint` trait.

This is the definition of the `Endpoint` trait, you need to specify the type of `Output` and implement the `call` method.

```rust
/// An HTTP request handler.
#[async_trait]
pub trait Endpoint: Send + Sync + 'static {
    /// Represents the response of the endpoint.
    type Output: IntoResponse;

    /// Get the response to the request.
    async fn call(&self, req: Request) -> Self::Output;
}
```

Now we implement an `Endpoint`, which receives HTTP requests and outputs a string containing the request method and path.

The `Output` associated type must be a type that implements the `IntoResponse` trait. Poem has been implemented by most
commonly used types.

Since `Endpoint` contains an asynchronous method `call`, we need to decorate it with the `async_trait` macro.

```rust
struct MyEndpoint;

#[async_trait]
impl Endpoint for MyEndpoint {
    type Output = String;
    
    async fn call(&self, req: Request) -> Self::Output {
        format!("method={} path={}", req.method(), req.uri().path());
    }
}
```

## Create from functions

You can use `poem::endpoint::make` and `poem::endpoint::make_sync` to create endpoints from asynchronous functions and
synchronous functions.

The following endpoint does the same thing:

```rust
let ep = poem::endpoint::make(|req| async move {
    format!("method={} path={}", req.method(), req.uri().path())
});
```

## EndpointExt

The `EndpointExt` trait provides some convenience functions for converting the input or output of the endpoint.

- `EndpointExt::before` is used to convert the request.
- `EndpointExt::after` is used to convert the output.
- `EndpointExt::map_ok`, `EndpointExt::map_err`, `EndpointExt::and_then` are used to process the output of type `Result<T>`.

## Using Result type

`Poem` also implements `IntoResponse` for the `poem::Result<T>` type, so it can also be used as the output type of the
endpoint, so you can use `?` in the `call` method.

```rust
struct MyEndpoint;

#[async_trait]
impl Endpoint for MyEndpoint {
    type Output = poem::Result<String>;
    
    async fn call(&self, req: Request) -> Self::Output {
        Ok(req.take_body().into_string().await?)
    }
}
```

You can use the `EndpointExt::map_to_response` method to convert the output of the endpoint to the `Response` type, or 
use the `EndpointExt::map_to_result` to convert the output to the `poem::Result<Response>` type.

```rust
let ep = MyEndpoint.map_to_response() // impl Endpoint<Output = Response>
```
