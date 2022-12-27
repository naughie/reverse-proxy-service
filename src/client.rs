//! Includes helper functions to build [`Client`]s, and some re-exports from [`hyper::client`] or
//! [`hyper_tls`].

use hyper::body::HttpBody;
pub use hyper::client::{Builder, Client};

use hyper::client::connect::Connect;
pub use hyper::client::connect::HttpConnector;

#[cfg(feature = "https")]
pub use hyper_tls::HttpsConnector;

/// Default [`Builder`].
pub fn builder() -> Builder {
    Builder::default()
}

/// Same as [`Client::new()`], except for the `B` parameter.
pub fn http_default<B>() -> Client<HttpConnector, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    Builder::default().build_http()
}

/// With the default [`HttpsConnector`].
#[cfg(feature = "https")]
pub fn https_default<B>() -> Client<HttpsConnector<HttpConnector>, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    Builder::default().build(HttpsConnector::new())
}

/// Default builder and given connector.
pub fn with_connector_default<C, B>(conn: C) -> Client<C, B>
where
    C: Connect + Clone,
    B: HttpBody + Send,
    B::Data: Send,
{
    Builder::default().build(conn)
}
