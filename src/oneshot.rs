use crate::client;
use crate::future::RevProxyFuture;
use crate::rewrite::PathRewriter;
use crate::Error;

use client::HttpConnector;
#[cfg(feature = "__rustls")]
use client::RustlsConnector;
#[cfg(feature = "nativetls")]
use hyper_tls::HttpsConnector as NativeTlsConnector;

use http::uri::{Authority, Scheme};
use http::Error as HttpError;
use http::{Request, Response};

//use hyper::body::{Body, HttpBody};
use hyper::body::{Body as HttpBody, Incoming};
use hyper_util::client::legacy::{connect::Connect, Client};

use tower_service::Service;

use std::convert::Infallible;
use std::task::{Context, Poll};

type BoxErr = Box<dyn std::error::Error + Send + Sync>;

/// A [`Service<Request<B>>`] that sends a request and returns the response, owning a [`Client`].
///
/// ```
/// # async fn run_test() {
/// # use reverse_proxy_service::OneshotService;
/// # use reverse_proxy_service::Static;
/// # use tower_service::Service;
/// # use hyper::body::Body;
/// # use http::Request;
/// let mut svc = OneshotService::http_default("example.com:1234", Static("bar")).unwrap();
/// let req = Request::builder()
///     .uri("https://myserver.com/foo")
///     .body(Body::empty())
///     .unwrap();
/// // http://example.com:1234/bar
/// let _res = svc.call(req).await.unwrap();
/// # }
/// ```
pub struct OneshotService<Pr, C = HttpConnector, B = Incoming> {
    client: Client<C, B>,
    scheme: Scheme,
    authority: Authority,
    path: Pr,
}

impl<Pr: Clone, C: Clone, B> Clone for OneshotService<Pr, C, B> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            scheme: self.scheme.clone(),
            authority: self.authority.clone(),
            path: self.path.clone(),
        }
    }
}

impl<Pr, C, B> OneshotService<Pr, C, B> {
    /// Initializes a service with a general `Client`.
    ///
    /// A client can be built by functions in [`client`].
    ///
    /// For the meaning of "scheme" and "authority", refer to the documentation of
    /// [`Uri`](http::uri::Uri).
    ///
    /// The `path` should implement [`PathRewriter`].
    pub fn from<S, A>(
        client: Client<C, B>,
        scheme: S,
        authority: A,
        path: Pr,
    ) -> Result<Self, HttpError>
    where
        Scheme: TryFrom<S>,
        <Scheme as TryFrom<S>>::Error: Into<HttpError>,
        Authority: TryFrom<A>,
        <Authority as TryFrom<A>>::Error: Into<HttpError>,
    {
        let scheme = scheme.try_into().map_err(Into::into)?;
        let authority = authority.try_into().map_err(Into::into)?;
        Ok(Self {
            client,
            scheme,
            authority,
            path,
        })
    }
}

impl<Pr, B> OneshotService<Pr, HttpConnector, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    /// Use [`client::http_default()`] to build a client.
    ///
    /// For the meaning of "authority", refer to the documentation of [`Uri`](http::uri::Uri).
    ///
    /// The `path` should implement [`PathRewriter`].
    pub fn http_default<A>(authority: A, path: Pr) -> Result<Self, HttpError>
    where
        Authority: TryFrom<A>,
        <Authority as TryFrom<A>>::Error: Into<HttpError>,
    {
        let authority = authority.try_into().map_err(Into::into)?;
        Ok(Self {
            client: client::http_default(),
            scheme: Scheme::HTTP,
            authority,
            path,
        })
    }
}

#[cfg(any(feature = "https", feature = "nativetls"))]
impl<Pr, B> OneshotService<Pr, NativeTlsConnector<HttpConnector>, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    /// Use [`client::https_default()`] to build a client.
    ///
    /// This is the same as [`Self::nativetls_default()`].
    ///
    /// For the meaning of "authority", refer to the documentation of [`Uri`](http::uri::Uri).
    ///
    /// The `path` should implement [`PathRewriter`].
    #[cfg_attr(docsrs, doc(cfg(any(feature = "https", feature = "nativetls"))))]
    pub fn https_default<A>(authority: A, path: Pr) -> Result<Self, HttpError>
    where
        Authority: TryFrom<A>,
        <Authority as TryFrom<A>>::Error: Into<HttpError>,
    {
        let authority = authority.try_into().map_err(Into::into)?;
        Ok(Self {
            client: client::https_default(),
            scheme: Scheme::HTTPS,
            authority,
            path,
        })
    }
}

#[cfg(feature = "nativetls")]
impl<Pr, B> OneshotService<Pr, NativeTlsConnector<HttpConnector>, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    /// Use [`client::nativetls_default()`] to build a client.
    ///
    /// For the meaning of "authority", refer to the documentation of [`Uri`](http::uri::Uri).
    ///
    /// The `path` should implement [`PathRewriter`].
    #[cfg_attr(docsrs, doc(cfg(feature = "nativetls")))]
    pub fn nativetls_default<A>(authority: A, path: Pr) -> Result<Self, HttpError>
    where
        Authority: TryFrom<A>,
        <Authority as TryFrom<A>>::Error: Into<HttpError>,
    {
        let authority = authority.try_into().map_err(Into::into)?;
        Ok(Self {
            client: client::nativetls_default(),
            scheme: Scheme::HTTPS,
            authority,
            path,
        })
    }
}

#[cfg(feature = "__rustls")]
impl<Pr, B> OneshotService<Pr, RustlsConnector<HttpConnector>, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    /// Use [`client::rustls_default()`] to build a client.
    ///
    /// For the meaning of "authority", refer to the documentation of [`Uri`](http::uri::Uri).
    ///
    /// The `path` should implement [`PathRewriter`].
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    pub fn https_default<A>(authority: A, path: Pr) -> Result<Self, HttpError>
    where
        Authority: TryFrom<A>,
        <Authority as TryFrom<A>>::Error: Into<HttpError>,
    {
        let authority = authority.try_into().map_err(Into::into)?;
        Ok(Self {
            client: client::rustls_default(),
            scheme: Scheme::HTTPS,
            authority,
            path,
        })
    }
}

impl<C, B, Pr> Service<Request<B>> for OneshotService<Pr, C, B>
where
    C: Connect + Clone + Send + Sync + 'static,
    B: HttpBody + Send + 'static + Unpin,
    B::Data: Send,
    B::Error: Into<BoxErr>,
    Pr: PathRewriter,
{
    type Response = Result<Response<Incoming>, Error>;
    type Error = Infallible;
    type Future = RevProxyFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        RevProxyFuture::new(
            &self.client,
            req,
            &self.scheme,
            &self.authority,
            &mut self.path,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_helper;
    use crate::ReplaceAll;

    use http::uri::{Parts, Uri};

    fn make_svc() -> OneshotService<ReplaceAll<'static>, HttpConnector, String> {
        let uri = Uri::try_from(&mockito::server_url());
        assert!(uri.is_ok());
        let uri = uri.unwrap();

        let Parts {
            scheme, authority, ..
        } = uri.into_parts();

        let svc = OneshotService::from(
            client::http_default(),
            scheme.unwrap(),
            authority.unwrap(),
            ReplaceAll("foo", "goo"),
        );
        assert!(svc.is_ok());
        svc.unwrap()
    }

    #[tokio::test]
    async fn match_path() {
        let mut svc = make_svc();
        test_helper::match_path(&mut svc).await;
    }

    #[tokio::test]
    async fn match_query() {
        let mut svc = make_svc();
        test_helper::match_query(&mut svc).await;
    }

    #[tokio::test]
    async fn match_post() {
        let mut svc = make_svc();
        test_helper::match_post(&mut svc).await;
    }

    #[tokio::test]
    async fn match_header() {
        let mut svc = make_svc();
        test_helper::match_header(&mut svc).await;
    }
}
