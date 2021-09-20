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

## Combine multiple listeners.

Call `Listener::combine` to combine two listeners into one, or you can call this function multiple times to combine more listeners.

```rust
let listener = TcpListener::bind("127.0.0.1:3000")
      .combine(TcpListener::bind("127.0.0.1:3001"))
      .combine(TcpListener::bind("127.0.0.1:3002"));
```