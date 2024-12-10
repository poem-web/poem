use prost::Name;

poem_grpc::include_proto!("helloworld");

fn main() -> Result<(), std::io::Error> {
    println!(
        "HelloRequest has {} full name and {} type url",
        HelloRequest::full_name(),
        HelloRequest::type_url()
    );
    println!(
        "HelloReply has {} full name and {} type url",
        HelloReply::full_name(),
        HelloReply::type_url()
    );

    Ok(())
}
