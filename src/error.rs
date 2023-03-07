use http::Error as HttpError;
use hyper::Error as HyperError;

#[cfg(feature = "axum")]
use axum::response::{IntoResponse, Response};
#[cfg(feature = "axum")]
use http::StatusCode;

use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    InvalidUri(HttpError),
    RequestFailed(HyperError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUri(e) => {
                write!(f, "Invalid uri: {e}")
            }
            Self::RequestFailed(e) => {
                write!(f, "Request failed: {e}")
            }
        }
    }
}

impl StdError for Error {}

#[cfg(feature = "axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        log::error!("{self}");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
