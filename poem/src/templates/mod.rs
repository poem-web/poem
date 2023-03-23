mod template; pub use template::Template;

#[cfg(feature = "templates")]
#[cfg_attr(docsrs, doc(cfg(feature = "templates")))]
pub mod tera;