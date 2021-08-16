mod starwars;

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Request, Response, Schema,
};
use poem::{
    middleware::AddData,
    prelude::*,
    web::{Data, Html, Json},
};
use starwars::{QueryRoot, StarWars, StarWarsSchema};

async fn graphql_handler(schema: Data<StarWarsSchema>, req: Json<Request>) -> Json<Response> {
    Json(schema.execute(req.0).await)
}

async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/")))
}

#[tokio::main]
async fn main() {
    // let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
    //     .data(StarWars::new())
    //     .finish();
    //
    // let app = route()
    //     .at("/", get(graphql_playground).post(graphql_handler))
    //     .with(AddData::new(schema));
    //
    // println!("Playground: http://localhost:3000");
    // serve(app).run("0.0.0.0:3000").await.unwrap();
}
