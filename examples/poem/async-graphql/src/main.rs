mod starwars;

use async_graphql::{
    EmptyMutation, EmptySubscription, Request, Response, Schema,
    http::{GraphQLPlaygroundConfig, playground_source},
};
use poem::{
    EndpointExt, IntoResponse, Route, Server, get, handler,
    listener::TcpListener,
    web::{Data, Html, Json},
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
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(StarWars::new())
        .finish();

    let app = Route::new()
        .at("/", get(graphql_playground).post(graphql_handler))
        .data(schema);

    println!("Playground: http://localhost:3000");

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
