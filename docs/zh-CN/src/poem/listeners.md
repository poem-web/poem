# 监听器

`Poem` 提供了一些常用的监听器。

- TcpListener

  侦听传入的 TCP 连接。

- UnixListener

  侦听传入的 Unix 域套接字连接。

## TLS

你可以调用`Listener::tls` 函数来包装一个侦听器并使其支持TLS 连接。
  
```rust
let listener = TcpListener::bind("127.0.0.1:3000")
    .tls(TlsConfig::new().key(KEY).cert(CERT));
```

## TLS 重载

你可以使用流将最新的 Tls 配置传递给 `Poem`。

以下示例每 1 分钟从文件中加载最新的 TLS 配置：

```rust
use async_trait::async_trait;

fn load_tls_config() -> Result<TlsConfig, std::io::Error> {
  Ok(TlsConfig::new()
          .cert(std::fs::read("cert.pem")?)
          .key(std::fs::read("key.pem")?))
}

let listener = TcpListener::bind("127.0.0.1:3000")
    .tls(async_stream::stream! {
        loop {
            if let Ok(tls_config) = load_tls_config() {
                yield tls_config;
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
```

## 组合多个监听器。

调用`Listener::combine`将两个监听器合二为一，也可以多次调用该函数来合并更多的监听器。

```rust
let listener = TcpListener::bind("127.0.0.1:3000")
      .combine(TcpListener::bind("127.0.0.1:3001"))
      .combine(TcpListener::bind("127.0.0.1:3002"));
```