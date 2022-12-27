use crate::rewrite::PathRewriter;
use crate::Error;

use http::uri::{Authority, Scheme};
use http::Error as HttpError;
use http::{Request, Response};

use hyper::body::{Body, HttpBody};
use hyper::client::{connect::Connect, Client, ResponseFuture};

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

type BoxErr = Box<dyn std::error::Error + Send + Sync>;

pub struct RevProxyFuture {
    inner: Result<ResponseFuture, Option<HttpError>>,
}

impl RevProxyFuture {
    pub(crate) fn new<C, B, Pr>(
        client: &Client<C, B>,
        mut req: Request<B>,
        scheme: &Scheme,
        authority: &Authority,
        path: &mut Pr,
    ) -> Self
    where
        C: Connect + Clone + Send + Sync + 'static,
        B: HttpBody + Send + 'static,
        B::Data: Send,
        B::Error: Into<BoxErr>,
        Pr: PathRewriter,
    {
        let inner = path
            .rewrite_uri(&mut req, scheme, authority)
            .map(|_| client.request(req))
            .map_err(Some);
        Self { inner }
    }
}

impl Future for RevProxyFuture {
    type Output = Result<Result<Response<Body>, Error>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.inner {
            Ok(fut) => match Future::poll(Pin::new(fut), cx) {
                Poll::Ready(res) => Poll::Ready(Ok(res.map_err(Error::RequestFailed))),
                Poll::Pending => Poll::Pending,
            },
            Err(e) => match e.take() {
                Some(e) => Poll::Ready(Ok(Err(Error::InvalidUri(e)))),
                None => unreachable!("RevProxyFuture::poll() is called after ready"),
            },
        }
    }
}
