use std::str::FromStr;

use http::header;
use smallvec::SmallVec;
use unic_langid::LanguageIdentifier;

use crate::{
    error::I18NError,
    i18n::{I18NArgs, I18NBundle, I18NResources},
    FromRequest, Request, RequestBody, Result,
};

type LanguageArray = SmallVec<[LanguageIdentifier; 8]>;

/// An extractor that parses the `Accept-Language` header and negotiates
/// language bundles.
///
/// # Example
///
/// ```
/// use poem::{
///     handler,
///     http::header,
///     i18n::{I18NResources, Locale},
///     test::TestClient,
///     Endpoint, EndpointExt, Request, Route,
/// };
///
/// let resources = I18NResources::builder()
///     .add_ftl("en-US", "hello-world = hello world!")
///     .add_ftl("zh-CN", "hello-world = 你好世界！")
///     .build()
///     .unwrap();
///
/// #[handler]
/// async fn index(locale: Locale) -> String {
///     locale
///         .text("hello-world")
///         .unwrap_or_else(|_| "error".to_string())
/// }
///
/// let app = Route::new().at("/", index).data(resources);
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli
///     .get("/")
///     .header(header::ACCEPT_LANGUAGE, "en-US")
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("hello world!").await;
///
/// let resp = cli
///     .get("/")
///     .header(header::ACCEPT_LANGUAGE, "zh-CN")
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("你好世界！").await;
/// # });
/// ```
pub struct Locale {
    bundle: I18NBundle,
}

impl Locale {
    /// Gets the text with arguments.
    ///
    /// See also: [`I18NBundle::text_with_args`](I18NBundle::text_with_args)
    pub fn text_with_args<'a>(
        &self,
        id: impl AsRef<str>,
        args: impl Into<I18NArgs<'a>>,
    ) -> Result<String, I18NError> {
        self.bundle.text_with_args(id, args)
    }

    /// Gets the text.
    ///
    /// See also: [`I18NBundle::text`](I18NBundle::text)
    pub fn text(&self, id: impl AsRef<str>) -> Result<String, I18NError> {
        self.bundle.text(id)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Locale {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let resources = req
            .extensions()
            .get::<I18NResources>()
            .expect("To use the `Locale` extractor, the `I18NResources` data is required.");

        let accept_languages = req
            .headers()
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .map(parse_accept_languages)
            .unwrap_or_default();

        Ok(Self {
            bundle: resources.negotiate_languages(&accept_languages),
        })
    }
}

fn parse_accept_languages(value: &str) -> LanguageArray {
    let mut languages = SmallVec::<[_; 8]>::new();

    for s in value.split(',').map(str::trim) {
        if let Some(res) = parse_language(s) {
            languages.push(res);
        }
    }

    languages.sort_by(|(_, a), (_, b)| b.cmp(a));
    languages
        .into_iter()
        .map(|(language, _)| language)
        .collect()
}

fn parse_language(value: &str) -> Option<(LanguageIdentifier, u16)> {
    let mut parts = value.split(';');
    let name = parts.next()?.trim();
    let quality = match parts.next() {
        Some(quality) => parse_quality(quality).unwrap_or_default(),
        None => 1000,
    };
    let language = LanguageIdentifier::from_str(name).ok()?;
    Some((language, quality))
}

fn parse_quality(value: &str) -> Option<u16> {
    let mut parts = value.split('=');
    let name = parts.next()?.trim();
    if name != "q" {
        return None;
    }
    let q = parts.next()?.trim().parse::<f32>().ok()?;
    Some((q.clamp(0.0, 1.0) * 1000.0) as u16)
}

#[cfg(test)]
mod tests {
    use unic_langid::langids;

    use super::*;

    #[test]
    fn test_parse_accept_languages() {
        assert_eq!(
            parse_accept_languages("zh-CN;q=0.5,en-US;q=0.7,fr;q=0.3").into_vec(),
            langids!("en-US", "zh-CN", "fr")
        );

        assert_eq!(
            parse_accept_languages("zh-CN ; q=0.5,en-US;q = 0.7,   fr;q=0.3").into_vec(),
            langids!("en-US", "zh-CN", "fr")
        );

        assert_eq!(
            parse_accept_languages("en-US;q=0.7,zh-CN,fr;q=0.3").into_vec(),
            langids!("zh-CN", "en-US", "fr")
        );
    }
}
