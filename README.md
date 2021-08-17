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
use poem::{get, handler, route, web::Path, Server};

#[handler]
async fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    let app = route().at("/hello/:name", get(hello));
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
```

More examples can be found [here][examples]. 

[examples]: https://github.com/poem-web/poem/tree/master/examples

## Contributing

:balloon: Thanks for your help improving the project! We are so happy to have you! 


## License

Licensed under either of

* Apache License, Version 2.0,
  ([LICENSE-APACHE](./LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)
  at your option.
* 

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Poem by you, shall be licensed as Apache, without any additional terms or conditions.
