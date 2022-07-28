use poem_grpc::Request;

poem_grpc::include_proto!("helloworld");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let client = GreeterClient::new("http://localhost:3000");
    let request = Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    let response = client.say_hello(request).await?;
    println!("RESPONSE={:?}", response);
    Ok(())
}
