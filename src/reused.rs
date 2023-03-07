use crate::client;
use crate::future::RevProxyFuture;
use crate::rewrite::PathRewriter;
use crate::Error;

use client::HttpConnector;
#[cfg(feature = "https")]
use client::HttpsConnector;

use http::uri::{Authority, Scheme};
use http::Error as HttpError;
use http::{Request, Response};

use hyper::body::{Body, HttpBody};
use hyper::client::{connect::Connect, Client};

use tower_service::Service;

use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};

type BoxErr = Box<dyn std::error::Error + Send + Sync>;

/// The return type of [`builder()`], [`builder_http()`] and [`builder_https()`].
#[derive(Debug)]
pub struct Builder<C = HttpConnector, B = Body> {
    client: Arc<Client<C, B>>,
    scheme: Scheme,
    authority: Authority,
}

impl<C, B> Clone for Builder<C, B> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            scheme: self.scheme.clone(),
            authority: self.authority.clone(),
        }
    }
}

impl<C, B> Builder<C, B> {
    pub fn build<Pr>(&self, path: Pr) -> ReusedService<Pr, C, B> {
        let Self {
            client,
            scheme,
            authority,
        } = Clone::clone(self);
        ReusedService {
            client,
            scheme,
            authority,
            path,
        }
    }
}

/// Builder of [`ReusedService`], with [`client::http_default()`].
///
/// For the meaning of "authority", refer to the documentation of [`Uri`](http::uri::Uri).
pub fn builder_http<B, A>(authority: A) -> Result<Builder<HttpConnector, B>, HttpError>
where
    B: HttpBody + Send,
    B::Data: Send,
    Authority: TryFrom<A>,
    <Authority as TryFrom<A>>::Error: Into<HttpError>,
{
    builder(client::http_default(), Scheme::HTTP, authority)
}

/// Builder of [`ReusedService`], with [`client::https_default()`].
///
/// For the meaning of "authority", refer to the documentation of [`Uri`](http::uri::Uri).
#[cfg(feature = "https")]
#[cfg_attr(docsrs, doc(cfg(feature = "https")))]
pub fn builder_https<B, A>(
    authority: A,
) -> Result<Builder<HttpsConnector<HttpConnector>, B>, HttpError>
where
    B: HttpBody + Send,
    B::Data: Send,
    Authority: TryFrom<A>,
    <Authority as TryFrom<A>>::Error: Into<HttpError>,
{
    builder(client::https_default(), Scheme::HTTP, authority)
}

/// Builder of [`ReusedService`].
///
/// For the meaning of "scheme" and "authority", refer to the documentation of
/// [`Uri`](http::uri::Uri).
pub fn builder<C, B, S, A>(
    client: Client<C, B>,
    scheme: S,
    authority: A,
) -> Result<Builder<C, B>, HttpError>
where
    Scheme: TryFrom<S>,
    <Scheme as TryFrom<S>>::Error: Into<HttpError>,
    Authority: TryFrom<A>,
    <Authority as TryFrom<A>>::Error: Into<HttpError>,
{
    let scheme = scheme.try_into().map_err(Into::into)?;
    let authority = authority.try_into().map_err(Into::into)?;
    Ok(Builder {
        client: Arc::new(client),
        scheme,
        authority,
    })
}

/// A [`Service<Request<B>>`] that sends a request and returns the response, sharing a [`Client`].
///
/// ```
/// # async fn run_test() {
/// # use reverse_proxy_service::ReusedService;
/// # use reverse_proxy_service::Static;
/// # use tower_service::Service;
/// # use hyper::body::Body;
/// # use http::Request;
/// let svc_builder = reverse_proxy_service::builder_http("example.com:1234").unwrap();
///
/// let mut svc1 = svc_builder.build(Static("bar"));
/// let mut svc2 = svc_builder.build(Static("baz"));
///
/// let req = Request::builder()
///     .uri("https://myserver.com/foo")
///     .body(Body::empty())
///     .unwrap();
/// // http://example.com:1234/bar
/// let _res = svc1.call(req).await.unwrap();
///
/// let req = Request::builder()
///     .uri("https://myserver.com/foo")
///     .body(Body::empty())
///     .unwrap();
/// // http://example.com:1234/baz
/// let _res = svc2.call(req).await.unwrap();
/// # }
/// ```
#[derive(Debug)]
pub struct ReusedService<Pr, C, B = Body> {
    client: Arc<Client<C, B>>,
    scheme: Scheme,
    authority: Authority,
    path: Pr,
}

impl<Pr: Clone, C, B> Clone for ReusedService<Pr, C, B> {
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

impl<Pr, C, B> ReusedService<Pr, C, B> {
    pub fn from<S, A>(
        client: Arc<Client<C, B>>,
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

impl<B, Pr> ReusedService<Pr, HttpConnector, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    pub fn http_default<A>(
        client: Arc<Client<HttpConnector, B>>,
        authority: A,
        path: Pr,
    ) -> Result<Self, HttpError>
    where
        Authority: TryFrom<A>,
        <Authority as TryFrom<A>>::Error: Into<HttpError>,
    {
        let authority = authority.try_into().map_err(Into::into)?;
        Ok(Self {
            client,
            scheme: Scheme::HTTP,
            authority,
            path,
        })
    }
}

#[cfg(feature = "https")]
impl<Pr, B> ReusedService<Pr, HttpsConnector<HttpConnector>, B>
where
    B: HttpBody + Send,
    B::Data: Send,
{
    #[cfg_attr(docsrs, doc(cfg(feature = "https")))]
    pub fn https_default<A>(
        client: Arc<Client<HttpsConnector<HttpConnector>, B>>,
        authority: A,
        path: Pr,
    ) -> Result<Self, HttpError>
    where
        Authority: TryFrom<A>,
        <Authority as TryFrom<A>>::Error: Into<HttpError>,
    {
        let authority = authority.try_into().map_err(Into::into)?;
        Ok(Self {
            client,
            scheme: Scheme::HTTPS,
            authority,
            path,
        })
    }
}

impl<C, B, Pr> Service<Request<B>> for ReusedService<Pr, C, B>
where
    C: Connect + Clone + Send + Sync + 'static,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxErr>,
    Pr: PathRewriter,
{
    type Response = Result<Response<Body>, Error>;
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

    fn make_svc() -> ReusedService<ReplaceAll<'static>, HttpConnector, String> {
        let uri = Uri::try_from(&mockito::server_url());
        assert!(uri.is_ok());
        let uri = uri.unwrap();

        let Parts {
            scheme, authority, ..
        } = uri.into_parts();

        let svc = ReusedService::from(
            Arc::new(client::http_default()),
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
