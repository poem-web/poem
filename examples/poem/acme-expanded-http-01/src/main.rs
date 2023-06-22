//! If you want to manage certificates yourself (sharing between servers,
//! sending over the network, etc) you can use this expanded ACME
//! certificate generation process which gives you access to the
//! generated certificates.
use std::{sync::Arc, time::Duration};

use poem::{
    get, handler,
    listener::{
        acme::{
            issue_cert, seconds_until_expiry, AcmeClient, ChallengeType, Http01Endpoint,
            Http01TokensMap, ResolveServerCert, ResolvedCertListener, LETS_ENCRYPT_PRODUCTION,
        },
        Listener, TcpListener,
    },
    middleware::Tracing,
    web::Path,
    EndpointExt, Route, RouteScheme, Server,
};
use tokio::{spawn, time::sleep};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let mut acme_client =
        AcmeClient::try_new(&LETS_ENCRYPT_PRODUCTION.parse().unwrap(), vec![]).await?;
    let cert_resolver = Arc::new(ResolveServerCert::default());
    let challenge = ChallengeType::Http01;
    let keys_for_http_challenge = Http01TokensMap::new();

    {
        let domains = vec!["poem.rs".to_string()];
        let keys_for_http_challenge = keys_for_http_challenge.clone();
        let cert_resolver = Arc::downgrade(&cert_resolver);
        spawn(async move {
            loop {
                let sleep_duration;
                if let Some(cert_resolver) = cert_resolver.upgrade() {
                    let cert = match issue_cert(
                        &mut acme_client,
                        &cert_resolver,
                        &domains,
                        challenge,
                        Some(&keys_for_http_challenge),
                    )
                    .await
                    {
                        Ok(result) => result.rustls_key,
                        Err(err) => {
                            eprintln!("failed to issue certificate: {}", err);
                            sleep(Duration::from_secs(60 * 5)).await;
                            continue;
                        }
                    };
                    sleep_duration = seconds_until_expiry(&cert) - 12 * 60 * 60;
                    *cert_resolver.cert.write() = Some(cert);
                } else {
                    break;
                }
                sleep(Duration::from_secs(sleep_duration as u64)).await;
            }
        });
    }

    let app = RouteScheme::new()
        .https(Route::new().at("/hello/:name", get(hello)))
        .http(Http01Endpoint {
            keys: keys_for_http_challenge,
        })
        .with(Tracing);

    Server::new(
        ResolvedCertListener::new(
            TcpListener::bind("0.0.0.0:443"),
            cert_resolver,
            ChallengeType::Http01,
        )
        .combine(TcpListener::bind("0.0.0.0:80")),
    )
    .name("hello-world")
    .run(app)
    .await
}
