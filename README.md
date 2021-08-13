# Poem

> A  program is like a poem, you cannot write a poem without writing it. --- Dijkstra 

A full-featured and easy-to-use web framework with
the Rust programming language. 

It is:

* **Fast**: Both Ease of use and performance.
* **Minimal generalization**: Minimizing the use of generics.


[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/poem.svg
[crates-url]: https://crates.io/crates/poem

## Example

```rust
use poem::middlewares::StripPrefix;
use poem::route::{self, Route};
use poem::EndpointExt;

async fn hello() -> &'static str {
    "hello"
}

#[tokio::main]
async fn main() {
    let route = Route::new().at("/hello", route::get(hello));
    let api = Route::new().at("/api/*", route.with(StripPrefix::new("/api")));

    poem::Server::new(api)
        .serve(&"127.0.0.1:3000".parse().unwrap())
        .await
        .unwrap();
}
```

More examples can be found [here][examples]. 

[examples]: https://github.com/poem-web/poem/tree/master/examples

## Contributing

:balloon: Thanks for your help improving the project! We are so happy to have you! 


## License

This project is licensed under the [Apache license].

[Apache license]: 

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Poem by you, shall be licensed as Apache, without any additional terms or conditions.
