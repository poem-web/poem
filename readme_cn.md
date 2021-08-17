<h1 align="center"><code>Poem Framework</code></h1>

<p align="center"><code>A  program is like a poem, you cannot write a poem without writing it. --- Dijkstra </code></p>
<p align="center">功能齐全且易于使用的 <code>Web</code> 框架，采用 <code>Rust</code> 编程语言。</p>
<p align="center">
    🏡<a href="https://poem-web.github.io/" target="_blank">HomePage</a> | 
    🇨🇳<a href="https://github.com/auula/poem/blob/master/readme_cn.md" target="_blank">中文说明</a> |
    🌎 <a href="https://github.com/auula/poem/blob/master/README.md">English</a>
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

## 特性
- 快速：兼具易用性和性能。
- 最小化泛化：最小化泛型的使用。

## 快速示例

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

查看更多例子[here][examples]. 

[examples]: https://github.com/poem-web/poem/tree/master/examples


## 开源协议

本项目获得的许可有:


* Apache License, Version 2.0,([LICENSE-APACHE](./LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)
  at your option.

 ## 贡献代码

🎈 我们欢迎更多开发者提`pr`贡献自己的代码，感谢您帮助改进项目！ 我们很高兴有你！你所提交的代码请注意使用的开源协议，并且附加许可条款或条件。