use hello_world::{greeter_client::GreeterClient, HelloRequest};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() {
    let mut client = GreeterClient::connect("http://localhost:3000")
        .await
        .unwrap();
    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    let response = client.say_hello(request).await.unwrap();
    println!("RESPONSE={:?}", response);
}
