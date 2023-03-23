use tera::Tera;

use crate::{
    error::{InternalServerError, IntoResult},
    web::Html,
    Endpoint, FromRequest, Middleware, Request, RequestBody, Result,
};

/// Tera Templating Middleware
pub struct TeraTemplatingMiddleware {
    tera: Tera,
}

impl TeraTemplatingMiddleware {
    /// Create a new instance of TeraTemplating, containing all the parsed
    /// templates found in the glob The errors are already handled. Use
    /// TeraTemplating::custom(tera: Tera) to modify tera settings.
    ///
    /// ```no_run
    /// use poem::tera::TeraTemplating;
    ///
    /// let templating = TeraTemplating::from_glob("templates/**/*");
    /// ```
    pub fn from_glob(glob: &str) -> Self {
        let tera = match Tera::new(glob) {
            Ok(t) => t,
            Err(e) => {
                // todo: move this up the stack via Result?
                tracing::debug!("Tera Parsing error: {e:?}");
                panic!("Failed to parse Tera template: {e}");
            }
        };

        Self { tera }
    }

    /// Create a new instance of TeraTemplating, containing all the parsed
    /// templates found in the directory. The errors are already handled. Use
    /// TeraTemplating::custom(tera: Tera) to modify tera settings.
    ///
    /// ```no_run
    /// use poem::tera::TeraTemplating;
    ///
    /// let templating = TeraTemplating::from_glob("templates");
    /// ```
    pub fn from_directory(template_directory: &str) -> Self {
        Self::from_glob(&format!("{template_directory}/**/*"))
    }

    /// Create a new instance of TeraTemplating, using the provided Tera
    /// instance
    ///
    /// ```no_run
    /// use poem::tera::{TeraTemplating, Tera};
    ///
    /// let mut tera = Tera::new("templates/**/*").expect("Failed to parse templates");
    ///
    /// tera.autoescape_on(vec![".html", ".sql"]);
    /// let templating = TeraTemplating::custom(tera);
    /// ```
    pub fn custom(tera: Tera) -> Self {
        Self { tera }
    }
}

impl Default for TeraTemplatingMiddleware {
    fn default() -> Self {
        Self::from_directory("templates")
    }
}

impl<E: Endpoint> Middleware<E> for TeraTemplatingMiddleware {
    type Output = TeraTemplatingEndpoint<E>;

    fn transform(&self, inner: E) -> Self::Output {
        Self::Output {
            tera: self.tera.clone(),
            inner,
            transformers: Vec::new(),
        }
    }
}

/// Tera Templating Endpoint
pub struct TeraTemplatingEndpoint<E> {
    tera: Tera,
    inner: E,
    transformers: Vec<fn(&mut Tera, &mut Request)>,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for TeraTemplatingEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let mut tera = self.tera.clone();

        for transformer in &self.transformers {
            transformer(&mut tera, &mut req);
        }

        req.set_data(tera);

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
        if let Err(err) = &self {
            tracing::error!("Failed to render Tera template: {err}");
            tracing::debug!("Tera Rendering error: {err:?}");
        }

        self.map_err(InternalServerError).map(Html)
    }
}

impl<E: Endpoint> TeraTemplatingEndpoint<E> {
    /// Add a transformer that apply changes to each tera instances (for
    /// instance, registering a dynamic filter) before passing tera to
    /// request handlers
    ///
    /// ```no_run
    /// use poem::{Route, EndpointExt, tera::TeraTemplating};
    ///
    /// let app = Route::new()
    ///     .with(TeraTemplating::from_glob("templates/**/*"))
    ///     .using(|tera, req| println!("{tera:?}\n{req:?}"));
    /// ```
    pub fn using(mut self, transformer: fn(&mut Tera, &mut Request)) -> Self {
        self.transformers.push(transformer);
        self
    }

    /// Enable live reloading only for debug mode (not for release)
    ///
    /// ```no_run
    /// use poem::{Route, EndpointExt, tera::TeraTemplating};
    ///
    /// let app = Route::new()
    ///     .with(TeraTemplating::from_glob("templates/**/*"))
    ///     .with_live_reloading();
    /// ```
    pub fn with_live_reloading(self) -> Self {
        #[cfg(debug_assertions)] {
            tracing::debug!("Live Reloading for Tera Templating is enabled");
        }

        self
    }
}
