use darling::FromMeta;
use proc_macro2::Ident;
use quote::{ToTokens, TokenStreamExt};

#[derive(FromMeta)]
pub(crate) enum ParameterStyle {
    Label,
    Matrix,
    Form,
    Simple,
    SpaceDelimited,
    PipeDelimited,
    DeepObject,
}

impl ToTokens for ParameterStyle {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = match self {
            ParameterStyle::Label => "Label",
            ParameterStyle::Matrix => "Matrix",
            ParameterStyle::Form => "Form",
            ParameterStyle::Simple => "Simple",
            ParameterStyle::SpaceDelimited => "SpaceDelimited",
            ParameterStyle::PipeDelimited => "PipeDelimited",
            ParameterStyle::DeepObject => "DeepObject",
        };

        tokens.append(Ident::new(name, proc_macro2::Span::call_site()));
    }
}
