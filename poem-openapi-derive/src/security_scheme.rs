use darling::{
    ast::{Data, Fields, Style},
    util::SpannedValue,
    FromDeriveInput, FromMeta, FromVariant,
};
use http::header::HeaderName;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{Attribute, DeriveInput, Error, Path};

use crate::{
    error::GeneratorResult,
    utils::{get_crate_name, get_description, optional_literal},
};

#[derive(FromMeta, Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum AuthType {
    #[darling(rename = "api_key")]
    ApiKey,
    #[darling(rename = "basic")]
    Basic,
    #[darling(rename = "bearer")]
    Bearer,
    #[darling(rename = "oauth2")]
    OAuth2,
    #[darling(rename = "openid_connect")]
    OpenIdConnect,
}

#[derive(FromMeta)]
struct OAuthFlow {
    #[darling(default)]
    authorization_url: Option<String>,
    #[darling(default)]
    token_url: Option<String>,
    #[darling(default)]
    refresh_url: Option<String>,
    #[darling(default)]
    scopes: Option<Path>,
}

impl OAuthFlow {
    fn generate_meta(&self, crate_name: &TokenStream) -> GeneratorResult<TokenStream> {
        let authorization_url = optional_literal(&self.authorization_url);
        let token_url = optional_literal(&self.token_url);
        let refresh_url = optional_literal(&self.refresh_url);
        let scopes = match &self.scopes {
            Some(scopes) => quote!(<#scopes as #crate_name::OAuthScopes>::meta()),
            None => quote!(::std::vec![]),
        };

        Ok(quote! {
            #crate_name::registry::MetaOAuthFlow {
                authorization_url: #authorization_url,
                token_url: #token_url,
                refresh_url: #refresh_url,
                scopes: #scopes,
            }
        })
    }
}

#[derive(FromMeta)]
struct OAuthFlows {
    #[darling(default)]
    implicit: Option<OAuthFlow>,
    #[darling(default)]
    password: Option<OAuthFlow>,
    #[darling(default)]
    client_credentials: Option<OAuthFlow>,
    #[darling(default)]
    authorization_code: Option<OAuthFlow>,
}

impl OAuthFlows {
    fn validate(&self, span: Span) -> GeneratorResult<()> {
        if self.implicit.is_none()
            && self.password.is_none()
            && self.authorization_code.is_none()
            && self.client_credentials.is_none()
        {
            return Err(Error::new(
                span,
                r#"At least one OAuth2 flow configuration is required."#,
            )
            .into());
        }

        if let Some(implicit) = &self.implicit {
            if implicit.authorization_url.is_none() {
                return Err(Error::new(
                    span,
                    r#"Missing authorization url. #[oai(authorization_url="...")]"#,
                )
                .into());
            }
        }

        if let Some(password) = &self.password {
            if password.token_url.is_none() {
                return Err(
                    Error::new(span, r#"Missing token url. #[oai(token_url="...")]"#).into(),
                );
            }
        }

        if let Some(client_credentials) = &self.client_credentials {
            if client_credentials.token_url.is_none() {
                return Err(
                    Error::new(span, r#"Missing token url. #[oai(token_url="...")]"#).into(),
                );
            }
        }

        if let Some(authorization_code) = &self.authorization_code {
            if authorization_code.authorization_url.is_none() {
                return Err(Error::new(
                    span,
                    r#"Missing authorization url. #[oai(authorization_url="...")]"#,
                )
                .into());
            }

            if authorization_code.token_url.is_none() {
                return Err(
                    Error::new(span, r#"Missing token url. #[oai(token_url="...")]"#).into(),
                );
            }
        }

        Ok(())
    }

    fn generate_meta(&self, crate_name: &TokenStream) -> GeneratorResult<TokenStream> {
        let implicit = match &self.implicit {
            Some(implicit) => {
                let meta = implicit.generate_meta(crate_name)?;
                quote!(::std::option::Option::Some(#meta))
            }
            None => quote!(::std::option::Option::None),
        };

        let password = match &self.password {
            Some(password) => {
                let meta = password.generate_meta(crate_name)?;
                quote!(::std::option::Option::Some(#meta))
            }
            None => quote!(::std::option::Option::None),
        };

        let client_credentials = match &self.client_credentials {
            Some(client_credentials) => {
                let meta = client_credentials.generate_meta(crate_name)?;
                quote!(::std::option::Option::Some(#meta))
            }
            None => quote!(::std::option::Option::None),
        };

        let authorization_code = match &self.authorization_code {
            Some(authorization_code) => {
                let meta = authorization_code.generate_meta(crate_name)?;
                quote!(::std::option::Option::Some(#meta))
            }
            None => quote!(::std::option::Option::None),
        };

        Ok(quote! {
            #crate_name::registry::MetaOAuthFlows {
                implicit: #implicit,
                password: #password,
                client_credentials: #client_credentials,
                authorization_code: #authorization_code,
            }
        })
    }
}

#[derive(FromMeta, Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum ApiKeyInType {
    #[darling(rename = "query")]
    Query,
    #[darling(rename = "header")]
    Header,
    #[darling(rename = "cookie")]
    Cookie,
}

#[derive(FromVariant)]
struct SecuritySchemeItem {
    ident: Ident,
    fields: Fields<syn::Type>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(oai), forward_attrs(doc))]
struct SecuritySchemeArgs {
    ident: Ident,
    data: Data<SecuritySchemeItem, syn::Type>,
    attrs: Vec<Attribute>,

    #[darling(default)]
    internal: bool,
    #[darling(default)]
    rename: Option<String>,
    ty: Option<AuthType>,
    #[darling(default)]
    key_in: Option<ApiKeyInType>,
    #[darling(default)]
    key_name: Option<SpannedValue<String>>,
    #[darling(default)]
    bearer_format: Option<String>,
    #[darling(default)]
    flows: Option<SpannedValue<OAuthFlows>>,
    #[darling(default)]
    openid_connect_url: Option<String>,
    #[darling(default)]
    checker: Option<Path>,
}

impl SecuritySchemeArgs {
    fn auth_type(&self) -> GeneratorResult<AuthType> {
        match self.ty {
            Some(ty) => Ok(ty),
            None => Err(Error::new_spanned(&self.ident, "Missing an auth type.").into()),
        }
    }

    fn validate(&self) -> GeneratorResult<()> {
        match self.auth_type()? {
            AuthType::ApiKey => self.validate_api_key(),
            AuthType::OAuth2 => self.validate_oauth2(),
            AuthType::OpenIdConnect => self.validate_openid_connect(),
            _ => Ok(()),
        }
    }

    fn validate_api_key(&self) -> GeneratorResult<()> {
        match &self.key_name {
            Some(name) => {
                HeaderName::try_from(&**name).map_err(|_| {
                    Error::new(
                        name.span(),
                        format!("`{}` is not a valid header name.", &**name),
                    )
                })?;
            }
            None => {
                return Err(Error::new_spanned(
                    &self.ident,
                    r#"Missing a key name. #[oai(key_name = "...")]"#,
                )
                .into())
            }
        }

        if self.key_in.is_none() {
            return Err(Error::new_spanned(
                &self.ident,
                r#"Missing a input type. #[oai(in = "...")]"#,
            )
            .into());
        }

        Ok(())
    }

    fn validate_oauth2(&self) -> GeneratorResult<()> {
        match &self.flows {
            Some(flows) => flows.validate(flows.span())?,
            None => {
                return Err(Error::new_spanned(
                    &self.ident,
                    r#"Missing an oauth2 flows. #[oai(flows = "...")]"#,
                )
                .into());
            }
        }

        Ok(())
    }

    fn validate_openid_connect(&self) -> GeneratorResult<()> {
        if self.openid_connect_url.is_none() {
            return Err(Error::new_spanned(
                &self.ident,
                r#"Missing open id connect url. #[oai(openid_connect_url = "...")]"#,
            )
            .into());
        }

        Ok(())
    }

    fn generate_register_security_scheme(
        &self,
        crate_name: &TokenStream,
        name: &str,
    ) -> GeneratorResult<TokenStream> {
        let description = get_description(&self.attrs)?;
        let description = optional_literal(&description);

        let key_name = match &self.key_name {
            Some(key_name) => {
                let name = &**key_name;
                quote!(::std::option::Option::Some(#name))
            }
            None => quote!(::std::option::Option::None),
        };
        let key_in = match &self.key_in {
            Some(ApiKeyInType::Query) => quote!(::std::option::Option::Some("query")),
            Some(ApiKeyInType::Header) => quote!(::std::option::Option::Some("header")),
            Some(ApiKeyInType::Cookie) => quote!(::std::option::Option::Some("cookie")),
            None => quote!(::std::option::Option::None),
        };
        let bearer_format = match &self.bearer_format {
            Some(bearer_format) => quote!(::std::option::Option::Some(#bearer_format)),
            None => quote!(::std::option::Option::None),
        };
        let openid_connect_url = match &self.openid_connect_url {
            Some(openid_connect_url) => quote!(::std::option::Option::Some(#openid_connect_url)),
            None => quote!(::std::option::Option::None),
        };

        let ts = match self.auth_type()? {
            AuthType::ApiKey => {
                quote! {
                    registry.create_security_scheme(#name, #crate_name::registry::MetaSecurityScheme {
                        ty: "apiKey",
                        description: #description,
                        name: #key_name,
                        key_in: #key_in,
                        scheme: ::std::option::Option::None,
                        bearer_format: ::std::option::Option::None,
                        flows: ::std::option::Option::None,
                        openid_connect_url: ::std::option::Option::None,
                    });
                }
            }
            AuthType::Basic => {
                quote! {
                    registry.create_security_scheme(#name, #crate_name::registry::MetaSecurityScheme {
                        ty: "http",
                        description: #description,
                        name: ::std::option::Option::None,
                        key_in: ::std::option::Option::None,
                        scheme: ::std::option::Option::Some("basic"),
                        bearer_format: #bearer_format,
                        flows: ::std::option::Option::None,
                        openid_connect_url: ::std::option::Option::None,
                    });
                }
            }
            AuthType::Bearer => {
                quote! {
                    registry.create_security_scheme(#name, #crate_name::registry::MetaSecurityScheme {
                        ty: "http",
                        description: #description,
                        name: ::std::option::Option::None,
                        key_in: ::std::option::Option::None,
                        scheme: ::std::option::Option::Some("bearer"),
                        bearer_format: #bearer_format,
                        flows: ::std::option::Option::None,
                        openid_connect_url: ::std::option::Option::None,
                    });
                }
            }
            AuthType::OAuth2 => {
                let flows = self.flows.as_ref().unwrap().generate_meta(crate_name)?;
                quote! {
                    registry.create_security_scheme(#name, #crate_name::registry::MetaSecurityScheme {
                        ty: "oauth2",
                        description: #description,
                        name: ::std::option::Option::None,
                        key_in: ::std::option::Option::None,
                        scheme: ::std::option::Option::None,
                        bearer_format: ::std::option::Option::None,
                        flows: ::std::option::Option::Some(#flows),
                        openid_connect_url: ::std::option::Option::None,
                    });
                }
            }
            AuthType::OpenIdConnect => {
                quote! {
                    registry.create_security_scheme(#name, #crate_name::registry::MetaSecurityScheme {
                        ty: "openIdConnect",
                        description: #description,
                        name: ::std::option::Option::None,
                        key_in: ::std::option::Option::None,
                        scheme: ::std::option::Option::None,
                        bearer_format: ::std::option::Option::None,
                        flows: ::std::option::Option::None,
                        openid_connect_url: #openid_connect_url,
                    });
                }
            }
        };
        Ok(ts)
    }

    fn generate_from_request(&self, crate_name: &TokenStream) -> GeneratorResult<TokenStream> {
        match self.auth_type()? {
            AuthType::ApiKey => {
                let key_name = self.key_name.as_ref().unwrap().as_str();
                let param_in = match self.key_in.as_ref().unwrap() {
                    ApiKeyInType::Query => quote!(#crate_name::registry::MetaParamIn::Query),
                    ApiKeyInType::Header => quote!(#crate_name::registry::MetaParamIn::Header),
                    ApiKeyInType::Cookie => quote!(#crate_name::registry::MetaParamIn::Cookie),
                };
                Ok(
                    quote!(<#crate_name::auth::ApiKey as #crate_name::auth::ApiKeyAuthorization>::from_request(req, query, #key_name, #param_in)),
                )
            }
            AuthType::Basic => Ok(
                quote!(<#crate_name::auth::Basic as #crate_name::auth::BasicAuthorization>::from_request(req)),
            ),
            AuthType::Bearer => Ok(
                quote!(<#crate_name::auth::Bearer as #crate_name::auth::BearerAuthorization>::from_request(req)),
            ),
            AuthType::OAuth2 => Ok(
                quote!(<#crate_name::auth::Bearer as #crate_name::auth::BearerAuthorization>::from_request(req)),
            ),
            AuthType::OpenIdConnect => Ok(
                quote!(<#crate_name::auth::Bearer as #crate_name::auth::BearerAuthorization>::from_request(req)),
            ),
        }
    }
}

pub(crate) fn generate(args: DeriveInput) -> GeneratorResult<TokenStream> {
    let args: SecuritySchemeArgs = SecuritySchemeArgs::from_derive_input(&args)?;
    let crate_name = get_crate_name(args.internal);
    let ident = &args.ident;

    match &args.data {
        Data::Struct(fields) => {
            let oai_typename = args.rename.clone().unwrap_or_else(|| ident.to_string());

            if fields.style != Style::Tuple || fields.fields.len() != 1 {
                return Err(Error::new_spanned(
                    ident,
                    "Only one unnamed field is allowed in the SecurityScheme struct.",
                )
                .into());
            }

            args.validate()?;

            let register_security_scheme =
                args.generate_register_security_scheme(&crate_name, &oai_typename)?;
            let from_request = args.generate_from_request(&crate_name)?;
            let path = args.checker.as_ref();

            let output = match path {
                Some(_) => quote! {
                    let output = #crate_name::__private::CheckerReturn::from(#path(&req, #from_request?).await).into_result()?;
                },
                None => quote! {
                    let output = #from_request?;
                },
            };

            let expanded = quote! {
                #[#crate_name::__private::poem::async_trait]
                impl<'a> #crate_name::ApiExtractor<'a> for #ident {
                    const TYPES: &'static [#crate_name::ApiExtractorType] = &[#crate_name::ApiExtractorType::SecurityScheme];

                    type ParamType = ();
                    type ParamRawType = ();

                    fn register(registry: &mut #crate_name::registry::Registry) {
                        #register_security_scheme
                    }

                    fn security_schemes() -> ::std::vec::Vec<&'static str> {
                        ::std::vec![#oai_typename]
                    }

                    async fn from_request(
                        req: &'a #crate_name::__private::poem::Request,
                        body: &mut #crate_name::__private::poem::RequestBody,
                        _param_opts: #crate_name::ExtractParamOptions<Self::ParamType>,
                    ) -> #crate_name::__private::poem::Result<Self> {
                        let query = req.extensions().get::<#crate_name::__private::UrlQuery>().unwrap();
                        #output
                        ::std::result::Result::Ok(Self(output))
                    }
                }
            };

            Ok(expanded)
        }
        Data::Enum(items) => {
            let mut registers = Vec::new();
            let mut security_schemes = Vec::new();
            let mut from_requests = Vec::new();

            if items.is_empty() {
                return Err(Error::new_spanned(ident, "At least one member is required.").into());
            }

            for item in items {
                if item.fields.style != Style::Tuple || item.fields.fields.len() != 1 {
                    return Err(Error::new_spanned(
                        ident,
                        "Only one unnamed field is allowed in the SecurityScheme enum.",
                    )
                    .into());
                }

                let item_ident = &item.ident;
                let item_type = &item.fields.fields[0];

                registers.push(
                    quote! { <#item_type as #crate_name::ApiExtractor>::register(registry); },
                );
                security_schemes.push(quote! {
                    security_schemes.extend(<#item_type as #crate_name::ApiExtractor>::security_schemes());
                });
                from_requests.push(quote! {
                    match <#item_type as #crate_name::ApiExtractor>::from_request(req, body, param_opts.clone()).await {
                        ::std::result::Result::Ok(item) => return Ok(#ident::#item_ident(item)),
                        ::std::result::Result::Err(err) => last_err = ::std::option::Option::Some(err),
                    }
                })
            }

            let expanded = quote! {
                #[#crate_name::__private::poem::async_trait]
                impl<'a> #crate_name::ApiExtractor<'a> for #ident {
                    const TYPES: &'static [#crate_name::ApiExtractorType] = &[#crate_name::ApiExtractorType::SecurityScheme];

                    type ParamType = ();
                    type ParamRawType = ();

                    fn register(registry: &mut #crate_name::registry::Registry) {
                        #(#registers)*
                    }

                    fn security_schemes() -> ::std::vec::Vec<&'static str> {
                        let mut security_schemes = ::std::vec![];
                        #(#security_schemes)*
                        security_schemes
                    }

                    async fn from_request(
                        req: &'a #crate_name::__private::poem::Request,
                        body: &mut #crate_name::__private::poem::RequestBody,
                        param_opts: #crate_name::ExtractParamOptions<Self::ParamType>,
                    ) -> #crate_name::__private::poem::Result<Self> {
                        let mut last_err = ::std::option::Option::None;
                        #(#from_requests)*
                        ::std::result::Result::Err(last_err.unwrap())
                    }
                }
            };

            Ok(expanded)
        }
    }
}
