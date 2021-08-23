mod starwars;

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Request, Response, Schema,
};
use poem::{
    handler,
    middleware::AddData,
    route,
    web::{Data, Html, Json},
    EndpointExt, IntoResponse, Server,
};
use starwars::{QueryRoot, StarWars, StarWarsSchema};

#[handler(method = "post")]
async fn graphql_handler(schema: Data<&StarWarsSchema>, req: Json<Request>) -> Json<Response> {
    Json(schema.execute(req.0).await)
}

#[handler(method = "get")]
fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/")))
}

#[tokio::main]
async fn main() {
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(StarWars::new())
        .finish();

    let app = route()
        .at("/", graphql_playground.or(graphql_handler))
        .with(AddData::new(schema));

    println!("Playground: http://localhost:3000");

    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
