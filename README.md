<h1 align="center">Poem Framework</h1>

<div align="center">
  <!-- CI -->
  <img src="https://github.com/poem-web/poem/workflows/CI/badge.svg" />
  <!-- codecov -->
  <img src="https://codecov.io/gh/poem-web/poem/branch/master/graph/badge.svg" />
  <a href="https://github.com/rust-secure-code/safety-dance/">
    <img src="https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat-square"
      alt="Unsafe Rust forbidden" />
  </a>
  <a href="https://blog.rust-lang.org/2021/11/01/Rust-1.61.0.html">
    <img src="https://img.shields.io/badge/rustc-1.61.0+-ab6000.svg"
      alt="rustc 1.61.0+" />
  </a>
  <a href="https://discord.gg/qWWNxwasb7">
    <img src="https://img.shields.io/discord/932986985604333638.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2" />
  </a>
  <a href="https://deps.rs/repo/github/poem-web/poem">
    <img src="https://img.shields.io/librariesio/release/cargo/poem.svg" />
  </a>
</div>
<p align="center"><code>A program is like a poem, you cannot write a poem without writing it. --- Dijkstra</code></p>
<p align="center"> A full-featured and easy-to-use web framework with the Rust programming language.</p>

***

This repo contains the following main components:

| Crate                                                                                                             | Description                    | Documentation                        | ChangeLog                                  |
|-------------------------------------------------------------------------------------------------------------------|--------------------------------|--------------------------------------|--------------------------------------------|
| **poem** [![](https://img.shields.io/crates/v/poem)](https://crates.io/crates/poem)                               | Poem Web                       | [(README)](poem/README.md)           | [(CHANGELOG)](poem/CHANGELOG.md)           |
| **poem-lambda** [![](https://img.shields.io/crates/v/poem-lambda)](https://crates.io/crates/poem-lambda)          | Poem for AWS Lambda            | [(README)](poem-lambda/README.md)    | [(CHANGELOG)](poem-lambda/CHANGELOG.md)    |
| **poem-openapi** [![](https://img.shields.io/crates/v/poem-openapi)](https://crates.io/crates/poem-openapi)       | OpenAPI for Poem Web           | [(README)](poem-openapi/README.md)   | [(CHANGELOG)](poem-openapi/CHANGELOG.md)   |
| **poem-dbsession** [![](https://img.shields.io/crates/v/poem-dbsession)](https://crates.io/crates/poem-dbsession) | Session storage using database | [(README)](poem-dbsession/README.md) | [(CHANGELOG)](poem-dbsession/CHANGELOG.md) |

***

The following are cases of community use:

| Repo                                                                             | Description                                                                                                            | Documentation                                                         |
|----------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------|
| [delicate](https://github.com/BinChengZhao/delicate)                             | A distributed task scheduling platform written in rust.                                                                | [(README)](https://delicate-rs.github.io/Roadmap.html)                |
| [databend](https://github.com/datafuselabs/databend)                             | A cloud-native data warehouse written in rust.                                                                         | [(ROADMAP)](https://github.com/datafuselabs/databend/issues/746)      |
| [muse](https://leihuo.163.com/)                                                  | A NetEase Leihuo's internal art resource sharing platform, backend in rust.                                            |                                                                       |
| [hik-proconnect](https://www.hikvision.com/en/products/software/hik-proconnect/) | A front-end automated deployment platform based on continuous integration of aws. Hik-ProConnect project for Hikvision |                                                                       |
| [warpgate](https://github.com/eugeny/warpgate)                                   | A smart SSH bastion host that works with any SSH clients.                                                              | [(README)](https://github.com/warp-tech/warpgate/blob/main/README.md) |
| [lust](https://github.com/ChillFish8/lust)                                       | A fast, auto-optimizing image server designed for high throughput and caching.                                         | [(README)](https://github.com/ChillFish8/lust/blob/master/README.md)  |
| [aptos](https://github.com/aptos-labs/aptos-core)                                | Building the safest and most scalable Layer 1 blockchain.                                                              | [(WEBSITE)](https://aptoslabs.com/)                                   |


### Startups

- [My Data My Consent](https://mydatamyconsent.com/) | Online data sharing for people and businesses simplified


### Resources

- [Examples](https://github.com/poem-web/poem/tree/master/examples)


## Contributing

:balloon: Thanks for your help improving the project! We are so happy to have you!


## License

Licensed under either of

* Apache License, Version 2.0,([LICENSE-APACHE](./LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)
  at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Poem by you, shall be licensed as Apache, without any additional terms or conditions.
