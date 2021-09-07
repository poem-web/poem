# Routing

The routing object is used to dispatch the request of the specified path and method to the specified endpoint.

The route object is actually an endpoint, which implements the `Endpoint` trait.

In the following example, we dispatch the requests of `/a` and `'b` to different endpoints.

```rust
#[handler]
async fn a() -> &'static str { "a" }

#[handler]
async fn b() -> &'static str { "b" }

let ep = poem::route()
    .at("/a", a)
    .at("/b", b);

Server::bind("127.0.0.1:3000").await?.run(ep).await
```

## Capture the variables

Use `:<name>` to capture the value of the specified segment in the path, or use `*<name>` to capture all the values after 
the specified prefix.

In the following example, the captured values will be stored in the variable `value`, and you can use the path extractor to get them.

```rust
#[handler]
async fn a(Path(String): Path<String>) {} 

let ep = poem::route()
    .at("/a/:value/b", handler)
    .at("/prefix/*value", handler);
```

## Nested

Sometimes we want to assign a path with a specified prefix to a specified endpoint, so that some functionally independent 
components can be created.

In the following example, the request path of the `hello` endpoint is `/api/hello`.

```rust
let api = poem::route().at("/hello", hello);
let ep = api.nest("/api", api);
```

Static file service is such an independent component.

```rust
let ep = route().nest("/files", Files::new("./static_files"));
```

## Method routing

The routing objects introduced above can only be dispatched by some specified paths, but dispatch by paths and methods 
is more common. `Poem` provides another route object `RouteMethod`, when it is combined with the `Route` object, it can 
provide this ability.

`Poem` provides some convenient functions to create `RouteMethod` objects, they are all named after HTTP standard methods.

```rust
use poem::route::{get, post};

let ep = poem::route()
    .at("/users", get(get_user).post(create_user).delete(delete_user).put(update_user));
```
