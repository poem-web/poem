use std::collections::BTreeMap;

use crate::{
    handler,
    http::{header, HeaderValue},
    session::Session,
    web::{cookie::Cookie, Path},
    Endpoint, IntoResponse, Request,
};

#[derive(Default)]
pub(crate) struct TestClient {
    cookies: BTreeMap<String, String>,
}

impl TestClient {
    pub(crate) async fn call(&mut self, ep: impl Endpoint, action: i32) {
        let mut req = Request::builder()
            .uri(format!("/{action}").parse().unwrap())
            .finish();

        let mut cookie = String::new();
        for (name, value) in &self.cookies {
            cookie += &format!("{name}={value};");
        }
        if !cookie.is_empty() {
            req.headers_mut()
                .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        }

        let resp = ep.call(req).await.unwrap().into_response();
        for s in resp.headers().get_all(header::SET_COOKIE) {
            if let Ok(s) = s.to_str() {
                let cookie = Cookie::parse(s).unwrap();

                if cookie.value_str().is_empty() {
                    self.cookies.remove(cookie.name());
                } else {
                    self.cookies
                        .insert(cookie.name().to_string(), cookie.value_str().to_string());
                }
            }
        }
    }

    pub(crate) fn assert_cookies<'a>(&self, cookies: impl IntoIterator<Item = (&'a str, &'a str)>) {
        assert_eq!(
            self.cookies,
            cookies
                .into_iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect::<BTreeMap<_, _>>()
        );
    }
}

#[handler(internal)]
pub(crate) fn index(Path(action): Path<i32>, session: &Session) {
    match action {
        1 => {
            session.set("a", 10);
            session.set("b", 20);
        }
        2 => {
            assert_eq!(session.get::<i32>("a"), Some(10));
            assert_eq!(session.get::<i32>("b"), Some(20));
            session.set("c", 30);
        }
        3 => {
            assert_eq!(session.get::<i32>("a"), Some(10));
            assert_eq!(session.get::<i32>("b"), Some(20));
            assert_eq!(session.get::<i32>("c"), Some(30));
            session.remove("b");
        }
        4 => {
            assert_eq!(session.get::<i32>("a"), Some(10));
            assert_eq!(session.get::<i32>("b"), None);
            assert_eq!(session.get::<i32>("c"), Some(30));
            session.clear();
        }
        5 => {
            assert!(session.is_empty());
            session.purge();
        }
        6 => {
            session.renew();
        }
        7 => {
            assert_eq!(session.get::<i32>("a"), Some(10));
            assert_eq!(session.get::<i32>("b"), Some(20));
            assert_eq!(session.get::<i32>("c"), Some(30));
        }
        _ => {}
    }
}
