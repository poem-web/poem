<h1 align="center"><code>Poem Framework</code></h1>

<p align="center"><code>A  program is like a poem, you cannot write a poem without writing it. --- Dijkstra </code></p>
<p align="center"> A full-featured and easy-to-use web framework with the Rust programming language.</p>
<p align="center"> 
    🏡<a href="https://poem-web.github.io/" target="_blank">HomePage</a> | 
    🇨🇳<a href="https://github.com/poem-web/poem/blob/master/readme_cn.md" target="_blank">中文说明</a> |
    🌎<a href="https://github.com/poem-web/poem/blob/master/README.md">English</a>
</p>
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
</div>

***

## Feature

* **Fast**: Both _Ease_ of use and performance.
* **Minimal generalization**: Minimizing the use of generics.

## Example

```rust
use poem::{handler, route, web::Path, RouteMethod, Server};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    let app = route().at("/hello/:name", RouteMethod::new().get(hello));
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
