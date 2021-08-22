use crate::{
    http::{header, StatusCode, Uri},
    IntoResponse, Response,
};

pub struct Redirect {
    status: StatusCode,
    uri: Uri,
}

impl Redirect {
    pub fn permanent(uri: Uri) -> Self {
        Self {
            status: StatusCode::PERMANENT_REDIRECT,
            uri,
        }
    }

    pub fn redirect(uri: Uri) -> Self {
        Self {
            status: StatusCode::MOVED_PERMANENTLY,
            uri,
        }
    }

    pub fn see_other(uri: Uri) -> Self {
        Self {
            status: StatusCode::SEE_OTHER,
            uri,
        }
    }

    pub fn temporary(uri: Uri) -> Self {
        Self {
            status: StatusCode::TEMPORARY_REDIRECT,
            uri,
        }
    }
}

impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        self.status
            .with_header(header::LOCATION, self.uri.to_string())
            .into_response()
    }
}
