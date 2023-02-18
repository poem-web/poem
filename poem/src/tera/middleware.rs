use tera::Tera;

use crate::{Endpoint, Middleware};
use super::TeraTemplatingEndpoint;

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