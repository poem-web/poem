<h1 align="center">MCP Server implementation for Poem</h1>

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/poem-mcpserver">
    <img src="https://img.shields.io/crates/v/poem-mcpserver.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/poem-mcpserver">
    <img src="https://img.shields.io/crates/d/poem-mcpserver.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/poem-mcpserver">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <a href="https://github.com/rust-secure-code/safety-dance/">
    <img src="https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat-square"
      alt="Unsafe Rust forbidden" />
  </a>
  <a>
    <img src="https://img.shields.io/badge/rustc-1.83.0+-ab6000.svg"
      alt="rustc 1.83.0+" />
  </a>
</div>

## Example

```toml
[dependencies]
poem-mcpserver.workspace = "*"
serde = { version = "1.0", features = ["derive"] }
schemars = "0.8.22"
```

```rust
use poem_mcpserver::{stdio::stdio, tool::Text, McpServer, Tools};

struct Counter {
    count: i32,
}

/// This server provides a counter tool that can increment and decrement values.
///
/// The counter starts at 0 and can be modified using the 'increment' and
/// 'decrement' tools. Use 'get_value' to check the current count.
#[Tools]
impl Counter {
    /// Increment the counter by 1
    async fn increment(&mut self) -> Text<i32> {
        self.count += 1;
        Text(self.count)
    }

    /// Decrement the counter by 1
    async fn decrement(&mut self) -> Text<i32> {
        self.count -= 1;
        Text(self.count)
    }

    /// Get the current counter value
    async fn get_value(&self) -> Text<i32> {
        Text(self.count)
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    stdio(McpServer::new().tools(Counter { count: 0 })).await
}
```

## Safety

This crate uses `#![forbid(unsafe_code)]` to ensure everything is implemented in 100% Safe Rust.

## MSRV

The minimum supported Rust version for this crate is `1.83.0`.

## Contributing

:balloon: Thanks for your help improving the project! We are so happy to have you!


## License

Licensed under either of

* Apache License, Version 2.0,([LICENSE-APACHE](./LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)
  at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Poem by you, shall be licensed as Apache, without any additional terms or conditions.
