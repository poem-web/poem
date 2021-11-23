mod starwars;

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Request, Response, Schema,
};
use poem::{
    get, handler,
    listener::TcpListener,
    web::{Data, Html, Json},
    EndpointExt, IntoResponse, Route, Server,
};
use starwars::{QueryRoot, StarWars, StarWarsSchema};

#[handler]
async fn graphql_handler(schema: Data<&StarWarsSchema>, req: Json<Request>) -> Json<Response> {
    Json(schema.execute(req.0).await)
}

#[handler]
fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/")))
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(StarWars::new())
        .finish();

    let app = Route::new()
        .at("/", get(graphql_playground).post(graphql_handler))
        .data(schema);

    println!("Playground: http://localhost:3000");

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
