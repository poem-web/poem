use tera::Tera;

use crate::{Endpoint, Middleware, error::IntoResult, Request, Result, FromRequest, RequestBody, error::InternalServerError, web::Html};

/// Tera Templating Middleware
pub struct TeraTemplatingMiddleware {
    tera: Tera
}

impl TeraTemplatingMiddleware {

    /// Create a new instance of TeraTemplating, containing all the parsed templates found in the glob
    /// The errors are already handled. Use TeraTemplating::custom(tera: Tera) to modify tera settings.
    ///
    /// ```no_compile
    /// use poem::tera::TeraTemplating;
    /// 
    /// let templating = TeraTemplating::from_glob("templates/**/*");
    /// ```
    pub fn from_glob(glob: &str) -> Self {
        let tera = match Tera::new(glob) {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {e}");
                ::std::process::exit(1);
            }
        };

        Self {
            tera
        }
    }

    /// Create a new instance of TeraTemplating, using the provided Tera instance
    ///
    /// ```no_compile
    /// use poem::tera::{TeraTemplating, Tera};
    /// 
    /// let mut tera = match Tera::new("templates/**/*") {
    ///     Ok(t) => t,
    ///     Err(e) => {
    ///         println!("Parsing error(s): {e}");
    ///         ::std::process::exit(1);
    ///     }
    /// };
    /// tera.autoescape_on(vec![".html", ".sql"]);
    /// let templating = TeraTemplating::custom(tera);
    /// ```
    pub fn custom(tera: Tera) -> Self {
        Self {
            tera
        }
    }
}

impl<E: Endpoint> Middleware<E> for TeraTemplatingMiddleware {
    type Output = TeraTemplatingEndpoint<E>;

    fn transform(&self, inner: E) -> Self::Output {
        Self::Output {
            tera: self.tera.clone(),
            inner,
        }
    }
}

/// Tera Templating Endpoint
pub struct TeraTemplatingEndpoint<E> {
    pub(super) tera: Tera,
    pub(super) inner: E
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for TeraTemplatingEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        req.extensions_mut().insert(self.tera.clone());

        self.inner.call(req).await
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Tera {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let tera = req
            .extensions()
            .get::<Tera>()
            .expect("To use the `Tera` extractor, the `TeraTemplating` endpoit is required.")
            .clone();

        Ok(tera)
    }
}

/// Shortcut (or not) for a Tera Templating handler Response
pub type TeraTemplatingResult = tera::Result<String>;

impl IntoResult<Html<String>> for TeraTemplatingResult {
    fn into_result(self) -> Result<Html<String>> {
        self
            .map_err(InternalServerError)
            .map(Html)
    }
}