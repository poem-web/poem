# Custom Payload

The purpose of this example is to show you how you can make your own custom payload "wrapper" types, similar to how you can accept and return JSON (for example).

For the example we implement a custom payload using [BCS](https://docs.rs/bcs/latest/bcs/). This example could theoretically used something else, such as bincode or yaml, there is nothing special about BCS itself for the purposes of this example.

## Running this example
Run the server:
```
cargo run
```

Send a request. We have included a file with data already in BCS format for this purpose:
```
curl -X POST localhost:3000/api/echo -H 'Content-Type: application/x-bcs' -H 'accept: application/x-bcs' -d "@data.bcs"
```
