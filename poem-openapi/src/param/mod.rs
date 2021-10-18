use std::{borrow::Cow, collections::HashMap};

use crate::{poem::Request, registry::MetaParamIn};

pub fn get<'a>(
    name: &str,
    in_type: MetaParamIn,
    request: &'a Request,
    query: &'a HashMap<String, String>,
) -> Option<Cow<'a, str>> {
    match in_type {
        MetaParamIn::Query => query.get(name).map(|s| s.as_str()).map(Cow::Borrowed),
        MetaParamIn::Header => request
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(Cow::Borrowed),
        MetaParamIn::Path => request.path_param(name).map(Cow::Borrowed),
        MetaParamIn::Cookie => request
            .cookie()
            .get(name)
            .as_ref()
            .map(|cookie| cookie.value_str().to_string())
            .map(Cow::Owned),
        MetaParamIn::CookiePrivate => request
            .cookie()
            .private()
            .get(name)
            .as_ref()
            .map(|cookie| cookie.value_str().to_string())
            .map(Cow::Owned),
        MetaParamIn::CookieSigned => request
            .cookie()
            .signed()
            .get(name)
            .as_ref()
            .map(|cookie| cookie.value_str().to_string())
            .map(Cow::Owned),
    }
}
