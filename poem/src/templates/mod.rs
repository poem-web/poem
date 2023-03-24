mod template; pub use template::Template;



#[cfg(feature = "templates")]
#[cfg_attr(docsrs, doc(cfg(feature = "templates")))]
pub mod tera;

#[cfg(feature = "live_reloading")]
#[cfg_attr(docsrs, doc(cfg(feature = "live_reloading")))]
mod live_reloading;

#[cfg(feature = "live_reloading")]
pub use live_reloading::LiveReloading;