<h1 align="center">Poem Framework</h1>

<p align="center"><code>A program is like a poem, you cannot write a poem without writing it. --- Dijkstra</code></p>
<p align="center"> A full-featured and easy-to-use web framework with the Rust programming language.</p>
<div align="center">
  <!-- CI -->
  <img src="https://github.com/poem-web/poem/workflows/CI/badge.svg" />
  <!-- codecov -->
  <img src="https://codecov.io/gh/poem-web/poem/branch/master/graph/badge.svg" />
  <!-- Crates version -->
  <a href="https://crates.io/crates/poem">
    <img src="https://img.shields.io/crates/v/poem.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/poem">
    <img src="https://img.shields.io/crates/d/poem.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/poem">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <a href="https://github.com/rust-secure-code/safety-dance/">
    <img src="https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat-square"
      alt="Unsafe Rust forbidden" />
  </a>
  <a href="https://blog.rust-lang.org/2021/07/29/Rust-1.54.0.html">
    <img src="https://img.shields.io/badge/rustc-1.54+-ab6000.svg"
      alt="rustc 1.54+" />
  </a>
</div>

***

* [Book](https://poem-web.github.io/poem/)
* [Docs](https://docs.rs/poem)
* [Cargo package](https://crates.io/crates/poem)

## Features

* **Fast**: Both _Ease_ of use and performance.
* **Minimal generalization**: Minimizing the use of generics.
* **Open API**: Use [poem-openapi](https://crates.io/crates/poem-openapi) to write APIs that comply with [OAS3](https://github.com/OAI/OpenAPI-Specification) specifications and automatically generate documents.

## Safety

This crate uses `#![forbid(unsafe_code)]` to ensure everything is implemented in 100% Safe Rust.

## Example

```rust
use poem::{handler, route, web::Path, route::get, Server};

#[handler]
fn hello(Path(name): Path<String>) -> String {
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

* Apache License, Version 2.0,([LICENSE-APACHE](./LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)
  at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Poem by you, shall be licensed as Apache, without any additional terms or conditions.
