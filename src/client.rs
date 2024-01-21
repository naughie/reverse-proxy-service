//! Includes helper functions to build [`Client`]s, and some re-exports from [`hyper::client`] or
//! [`hyper_tls`].
//!
use hyper::body::Body as HttpBody;
pub use hyper_util::client::legacy::{Builder, Client};

use hyper_util::client::legacy::connect::Connect;
pub use hyper_util::client::legacy::connect::HttpConnector;

#[cfg(feature = "https")]
#[cfg_attr(docsrs, doc(cfg(feature = "https")))]
pub use hyper_tls::HttpsConnector;

#[cfg(feature = "__rustls")]
#[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
pub use hyper_rustls::HttpsConnector as RustlsConnector;

#[cfg(feature = "nativetls")]
#[cfg_attr(docsrs, doc(cfg(feature = "nativetls")))]
pub use hyper_tls::HttpsConnector as NativeTlsConnector;

/// Default [`Builder`].
pub fn builder() -> Builder {
    Builder::new(hyper_util::rt::TokioExecutor::new())
}

/// Same as [`Client::new()`], except for the `B` parameter.
pub fn http_default<B>() -> Client<HttpConnector, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    Builder::new(hyper_util::rt::TokioExecutor::new()).build_http()
}

/// Alias to [`nativetls_default()`].
#[cfg(any(feature = "https", feature = "nativetls"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "https", feature = "nativetls"))))]
#[inline]
pub fn https_default<B>() -> Client<NativeTlsConnector<HttpConnector>, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    nativetls_default()
}

/// With the default [`hyper_tls::HttpsConnector`].
#[cfg(feature = "nativetls")]
#[cfg_attr(docsrs, doc(cfg(feature = "nativetls")))]
pub fn nativetls_default<B>() -> Client<NativeTlsConnector<HttpConnector>, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    Builder::default().build(NativeTlsConnector::new())
}

/// With the default [`hyper_rustls::HttpsConnector`].
///
/// The config is determined as follows. I think the cert root is similar to the `reqwest` crate.
///
/// 1. Cert roots
///
/// - if the feature `rustls-webpki-roots` is enabled, then use
/// [`HttpsConnector::with_webpki_roots()`](hyper_rustls::HttpsConnector::with_webpki_roots());
/// - if `rustls-webpki-roots` is disabled and `rustls-native-roots` enabled, then
/// [`HttpsConnector::with_native_roots()`](hyper_rustls::HttpsConnector::with_native_roots());
/// - otherwise compilation fails.
///
/// The feature `rustls` is equivalent to `rustls-webpki-roots`.
///
/// 2. Scheme
///
/// HTTPS only
///
/// 3. HTTP version
///
/// - if the feature `http1` is enabled, then call
/// [`HttpsConnector::enable_http1()`](hyper_rustls::HttpsConnector::enable_http1());
/// - if the feature `rustls-http2` is enabled, then call
/// [`HttpsConnector::enable_http2()`](hyper_rustls::HttpsConnector::enable_http2()).
///
/// This is not exclusive: if both features are enabled, then both mehtods are called.
///
#[cfg(feature = "__rustls")]
#[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
pub fn rustls_default<B>() -> Client<RustlsConnector<HttpConnector>, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    let conn = hyper_rustls::HttpsConnectorBuilder::new();
    #[cfg(feature = "rustls-webpki-roots")]
    let conn = conn.with_webpki_roots();
    #[cfg(all(not(feature = "rustls-webpki-roots"), feature = "rustls-native-roots"))]
    let conn = conn.with_native_roots();
    let conn = conn.https_only();
    #[cfg(feature = "http1")]
    let conn = conn.enable_http1();
    #[cfg(feature = "rustls-http2")]
    let conn = conn.enable_http2();
    Builder::default().build(conn.build())
}

/// Default builder and given connector.
pub fn with_connector_default<C, B>(conn: C) -> Client<C, B>
where
    C: Connect + Clone,
    B: HttpBody + Send,
    B::Data: Send,
{
    Builder::new(hyper_util::rt::TokioExecutor::new()).build(conn)
}
