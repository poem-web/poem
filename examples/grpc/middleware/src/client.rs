mod middleware;

use poem_grpc::{ClientConfig, Request};

poem_grpc::include_proto!("helloworld");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let client = GreeterClient::new(
        ClientConfig::builder()
            .uri("http://localhost:3000")
            .build()
            .unwrap(),
    );
    let request = Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    let response = client.say_hello(request).await?;
    println!("RESPONSE={response:?}");
    Ok(())
}
