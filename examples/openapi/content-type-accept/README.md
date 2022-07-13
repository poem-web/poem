# Example: Content Type + Accept

The purpose of this example is to demonstrate how to use the following two features of HTTP:

- `Content-Type`: When a client sends the server a request, it can use this to tell it what type the data is.
- `Accept`: When a client sends the server a request, it can use this to tell it what type of data it would like to receive as the response.

This allows a client to, for example, submit a request as JSON, but expect the response as XML, assuming the server supports both types.

To see what kind of spec this code produces, `cargo run` and then visit http://localhost:3000/spec.
