use poem::{
    get, handler,
    http::StatusCode,
    listener::TcpListener,
    middleware::Csrf,
    web::{CsrfToken, CsrfVerifier, Form, Html},
    EndpointExt, Error, IntoResponse, Result, Route, Server,
};
use serde::Deserialize;

#[handler]
async fn login_ui(token: &CsrfToken) -> impl IntoResponse {
    Html(format!(
        r#"
    <!DOCTYPE html>
    <html>
    <head><meta charset="UTF-8"><title>Example CSRF</title></head>
    <body>
    <form action="/" method="post">
        <input type="hidden" name="csrf_token" value="{}" />
        <div>
            <label>Username:<input type="text" name="username" /></label>
        </div>
        <div>
            <label>Password:<input type="password" name="password" /></label>
        </div>
        <button type="submit">Login</button>
    </form>
    </body>
    </html>
    "#,
        token.0
    ))
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct LoginRequest {
    csrf_token: String,
    username: String,
    password: String,
}

#[handler]
async fn login(verifier: &CsrfVerifier, Form(req): Form<LoginRequest>) -> Result<String> {
    if !verifier.is_valid(&req.csrf_token) {
        return Err(Error::from_status(StatusCode::UNAUTHORIZED));
    }

    Ok(format!("login success: {}", req.username))
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/", get(login_ui).post(login))
        .with(Csrf::new());
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
