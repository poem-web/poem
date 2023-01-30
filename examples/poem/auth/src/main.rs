use poem::{
    get, handler,
    http::{header, StatusCode},
    listener::TcpListener,
    session::{CookieConfig, CookieSession, Session},
    web::{Form, Html},
    EndpointExt, IntoResponse, Response, Result, Route, Server,
};
use serde::Deserialize;

#[handler]
fn signin_ui() -> impl IntoResponse {
    Html(
        r#"
    <!DOCTYPE html>
    <html>
    <head><meta charset="UTF-8"><title>Example Session Auth</title></head>
    <body>
    <form action="/signin" method="post">
        <div>
            <label>Username:<input type="text" name="username" value="test" /></label>
        </div>
        <div>
            <label>Password:<input type="password" name="password" value="123456" /></label>
        </div>
        <button type="submit">Login</button>
    </form>
    </body>
    </html>
    "#,
    )
}

#[derive(Deserialize)]
struct SigninParams {
    username: String,
    password: String,
}

#[handler]
fn signin(Form(params): Form<SigninParams>, session: &Session) -> impl IntoResponse {
    if params.username == "test" && params.password == "123456" {
        session.set("username", params.username);
        Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, "/")
            .finish()
    } else {
        Html(
            r#"
    <!DOCTYPE html>
    <html>
    <head><meta charset="UTF-8"><title>Example Session Auth</title></head>
    <body>
    no such user
    </body>
    </html>
    "#,
        )
        .into_response()
    }
}

#[handler]
fn index(session: &Session) -> impl IntoResponse {
    match session.get::<String>("username") {
        Some(username) => Html(format!(
            r#"
    <!DOCTYPE html>
    <html>
    <head><meta charset="UTF-8"><title>Example Session Auth</title></head>
    <body>
    <div>hello {username}, you are viewing a restricted resource</div>
    <a href="/logout">click here to logout</a>
    </body>
    </html>
    "#
        ))
        .into_response(),
        None => Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, "/signin")
            .finish(),
    }
}

#[handler]
fn logout(session: &Session) -> impl IntoResponse {
    session.purge();
    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/signin")
        .finish()
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/", get(index))
        .at("/signin", get(signin_ui).post(signin))
        .at("/logout", get(logout))
        .with(CookieSession::new(CookieConfig::new()));
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
