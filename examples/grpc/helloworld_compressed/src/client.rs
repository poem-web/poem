use poem_grpc::{ClientConfig, CompressionEncoding, Request};

poem_grpc::include_proto!("helloworld");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut client = GreeterClient::new(
        ClientConfig::builder()
            .uri("http://localhost:3000")
            .build()
            .unwrap(),
    );
    client.set_send_compressed(CompressionEncoding::GZIP);
    client.set_accept_compressed([CompressionEncoding::GZIP]);

    let request = Request::new(HelloRequest {
        name: "Poem".into(),
    });
    let response = client.say_hello(request).await?;
    println!("RESPONSE={response:?}");
    Ok(())
}
