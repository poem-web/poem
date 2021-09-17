use proc_macro2::TokenStream;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum GeneratorError {
    #[error("{0}")]
    Syn(#[from] syn::Error),

    #[error("{0}")]
    Darling(#[from] darling::Error),
}

impl GeneratorError {
    pub(crate) fn write_errors(self) -> TokenStream {
        match self {
            GeneratorError::Syn(err) => err.to_compile_error(),
            GeneratorError::Darling(err) => err.write_errors(),
        }
    }
}

pub(crate) type GeneratorResult<T> = std::result::Result<T, GeneratorError>;
