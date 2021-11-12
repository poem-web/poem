# Listeners

`Poem` provides some commonly used listeners.

- TcpListener

  Listens for incoming TCP connections.

- UnixListener

  Listens for incoming Unix domain socket connections.

## TLS

You can call the `Listener::tls` function to wrap a listener and make it support TLS connections.
  
```rust
let listener = TcpListener::bind("127.0.0.1:3000")
    .tls(TlsConfig::new().key(KEY).cert(CERT));
```

## TLS reload

You can use a stream to pass the latest Tls config to `Poem`.

The following example loads the latest TLS config from file every 1 minute:

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

## Combine multiple listeners.

Call `Listener::combine` to combine two listeners into one, or you can call this function multiple times to combine more listeners.

```rust
let listener = TcpListener::bind("127.0.0.1:3000")
      .combine(TcpListener::bind("127.0.0.1:3001"))
      .combine(TcpListener::bind("127.0.0.1:3002"));
```