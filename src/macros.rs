macro_rules! define_simple_errors {
    ($($(#[$docs:meta])* ($name:ident, $status:ident, $err_msg:literal);)*) => {
        $(
        $(#[$docs])*
        #[derive(Copy, Clone, Eq, PartialEq)]
        struct $name;

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", $err_msg)
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", $err_msg)
            }
        }

        impl crate::ResponseError for $name {
            fn as_response(&self) -> crate::Response {
                crate::Response::builder().status(crate::http::StatusCode::$status).body($err_msg.into())
            }
        }
        )*
    };
}
