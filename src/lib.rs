#![cfg_attr(docsrs, feature(doc_cfg))]

//! `reverse-proxy-service` is tower [`Service`s](tower_service::Service) that performs "reverse
//! proxy" with various rewriting rules.
//!
//! Internally these services use [`hyper::Client`] to send an incoming request to the another
//! server. The [`connector`](hyper::client::connect::Connect) for a client can be
//! [`HttpConnector`](hyper::client::HttpConnector), [`HttpsConnector`](hyper_tls::HttpsConnector),
//! or any ones whichever you want.
//!
//! # Examples
//!
//! There are two types of services, [`OneshotService`] and [`ReusedService`]. The
//! [`OneshotService`] *owns* the `Client`, while the [`ReusedService`] *shares* the `Client`
//! via [`Arc`](std::sync::Arc).
//!
//!
//! ## General usage
//!
//! ```
//! # async fn run_test() {
//! use reverse_proxy_service::ReusedServiceBuilder;
//! use reverse_proxy_service::{ReplaceAll, ReplaceN};
//!
//! use hyper::body::Body;
//! use http::Request;
//! use tower_service::Service as _;
//!
//! let svc_builder = reverse_proxy_service::builder_http("example.com:1234").unwrap();
//!
//! let req1 = Request::builder()
//!     .method("GET")
//!     .uri("https://myserver.com/foo/bar/foo")
//!     .body(Body::empty())
//!     .unwrap();
//!
//! // Clones Arc<Client>
//! let mut svc1 = svc_builder.build(ReplaceAll("foo", "baz"));
//! // http://example.com:1234/baz/bar/baz
//! let _res = svc1.call(req1).await.unwrap();
//!
//! let req2 = Request::builder()
//!     .method("POST")
//!     .uri("https://myserver.com/foo/bar/foo")
//!     .header("Content-Type", "application/x-www-form-urlencoded")
//!     .body(Body::from("key=value"))
//!     .unwrap();
//!
//! let mut svc2 = svc_builder.build(ReplaceN("foo", "baz", 1));
//! // http://example.com:1234/baz/bar/foo
//! let _res = svc2.call(req2).await.unwrap();
//! # }
//! ```
//!
//! In this example, the `svc1` and `svc2` shares the same `Client`, holding the `Arc<Client>`s
//! inside them.
//!
//! For more information of rewriting rules (`ReplaceAll`, `ReplaceN` *etc.*), see the
//! documentations of [`rewrite`].
//!
//!
//! ## With axum
//!
//! ```no_run
//! use reverse_proxy_service::ReusedServiceBuilder;
//! use reverse_proxy_service::{TrimPrefix, AppendSuffix, Static};
//!
//! use axum::Router;
//!
//! #[tokio::main]
//! async fn main() {
//!     let host1 = reverse_proxy_service::builder_http("example.com").unwrap();
//!     let host2 = reverse_proxy_service::builder_http("example.net:1234").unwrap();
//!
//!     let app = Router::new()
//!         .route_service("/healthcheck", host1.build(Static("/")))
//!         .route_service("/users/*path", host1.build(TrimPrefix("/users")))
//!         .route_service("/posts", host2.build(AppendSuffix("/")));
//!
//!     axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
//!         .serve(app.into_make_service())
//!         .await
//!         .unwrap();
//! }
//! ```
//!
//!
//! # Return Types
//!
//! The return type ([`Future::Output`](std::future::Future::Output)) of [`ReusedService`] and
//! [`OneshotService`] is `Result<Result<Response, Error>, Infallible>`. This is because axum's
//! [`Router`](axum::Router) accepts only such `Service`s.
//!
//! The [`Error`] type implements [`IntoResponse`](axum::response::IntoResponse) if you enable the
//! `axum`feature.
//! It returns an empty body, with the status code `INTERNAL_SERVER_ERROR`. The description of this
//! error will be logged out at [error](`log::error`) level in the
//! [`into_response()`](axum::response::IntoResponse::into_response()) method.
//!
//!
//! # Features
//!
//! By default only `http1` is enabled.
//!
//! - `http1`: uses `hyper/http1`
//! - `http2`: uses `hyper/http2`
//! - `https`: alias to `nativetls`
//! - `nativetls`: uses the `hyper-tls` crate
//! - `rustls`: alias to `rustls-webpki-roots`
//! - `rustls-webpki-roots`: uses the `hyper-rustls` crate, with the feature `webpki-roots`
//! - `rustls-native-roots`: uses the `hyper-rustls` crate, with the feature `rustls-native-certs`
//! - `rustls-http2`: `http2` plus `rustls`, and `rustls/http2` is enabled
//! - `axum`: implements [`IntoResponse`](axum::response::IntoResponse) for [`Error`]
//!
//! You must turn on either `http1`or `http2`. You cannot use the services if, for example, only
//! the `https` feature is on.
//!
//! Through this document, we use `rustls` to mean *any* of `rustls*` features unless otherwise
//! specified.

mod error;
pub use error::Error;

#[cfg(any(feature = "http1", feature = "http2"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "http1", feature = "http2"))))]
pub mod client;

pub mod rewrite;
pub use rewrite::*;

mod future;
pub use future::RevProxyFuture;

#[cfg(any(feature = "http1", feature = "http2"))]
mod oneshot;
#[cfg(any(feature = "http1", feature = "http2"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "http1", feature = "http2"))))]
pub use oneshot::OneshotService;

#[cfg(any(feature = "http1", feature = "http2"))]
mod reused;
#[cfg(all(
    any(feature = "http1", feature = "http2"),
    any(feature = "https", feature = "nativetls")
))]
#[cfg_attr(
    docsrs,
    doc(cfg(all(
        any(feature = "http1", feature = "http2"),
        any(feature = "https", feature = "nativetls")
    )))
)]
pub use reused::builder_https;
#[cfg(all(any(feature = "http1", feature = "http2"), feature = "nativetls"))]
#[cfg_attr(
    docsrs,
    doc(cfg(all(any(feature = "http1", feature = "http2"), feature = "nativetls")))
)]
pub use reused::builder_nativetls;
#[cfg(all(any(feature = "http1", feature = "http2"), feature = "__rustls"))]
#[cfg_attr(
    docsrs,
    doc(cfg(all(any(feature = "http1", feature = "http2"), feature = "rustls")))
)]
pub use reused::builder_rustls;
#[cfg(any(feature = "http1", feature = "http2"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "http1", feature = "http2"))))]
pub use reused::Builder as ReusedServiceBuilder;
#[cfg(any(feature = "http1", feature = "http2"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "http1", feature = "http2"))))]
pub use reused::ReusedService;
#[cfg(any(feature = "http1", feature = "http2"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "http1", feature = "http2"))))]
pub use reused::{builder, builder_http};

#[cfg(test)]
mod test_helper {
    use super::{Error, RevProxyFuture};
    use std::convert::Infallible;

    use http::StatusCode;
    use http::{Request, Response};

    use hyper::body::Incoming;

    use http_body_util::BodyExt;

    use tower_service::Service;

    use mockito::Matcher;

    async fn call<S, B>(
        svc: &mut S,
        req: (&str, &str, Option<&str>, B),
        expected: (StatusCode, &str),
    ) where
        S: Service<
            Request<String>,
            Response = Result<Response<Incoming>, Error>,
            Error = Infallible,
            Future = RevProxyFuture,
        >,
        B: Into<String>,
    {
        let req = if let Some(content_type) = req.2 {
            Request::builder()
                .method(req.0)
                .uri(format!("https://test.com{}", req.1))
                .header("Content-Type", content_type)
                .body(req.3.into())
        } else {
            Request::builder()
                .method(req.0)
                .uri(format!("https://test.com{}", req.1))
                .uri(format!("https://test.com{}", req.1))
                .body(req.3.into())
        }
        .unwrap();
        let res = svc.call(req).await.unwrap();
        assert!(res.is_ok());
        let res = res.unwrap();
        assert_eq!(res.status(), expected.0);
        let res = res.into_body().collect().await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap().to_bytes(), expected.1);
    }

    pub async fn match_path<S>(svc: &mut S)
    where
        S: Service<
            Request<String>,
            Response = Result<Response<Incoming>, Error>,
            Error = Infallible,
            Future = RevProxyFuture,
        >,
    {
        let _mk = mockito::mock("GET", "/goo/bar/goo/baz/goo")
            .with_body("ok")
            .create();

        call(
            svc,
            ("GET", "/foo/bar/foo/baz/foo", None, ""),
            (StatusCode::OK, "ok"),
        )
        .await;

        call(
            svc,
            ("GET", "/foo/bar/foo/baz", None, ""),
            (StatusCode::NOT_IMPLEMENTED, ""),
        )
        .await;
    }

    pub async fn match_query<S>(svc: &mut S)
    where
        S: Service<
            Request<String>,
            Response = Result<Response<Incoming>, Error>,
            Error = Infallible,
            Future = RevProxyFuture,
        >,
    {
        let _mk = mockito::mock("GET", "/goo")
            .match_query(Matcher::UrlEncoded("greeting".into(), "good day".into()))
            .with_body("ok")
            .create();

        call(
            svc,
            ("GET", "/foo?greeting=good%20day", None, ""),
            (StatusCode::OK, "ok"),
        )
        .await;

        call(
            svc,
            ("GET", "/foo", None, ""),
            (StatusCode::NOT_IMPLEMENTED, ""),
        )
        .await;
    }

    pub async fn match_post<S>(svc: &mut S)
    where
        S: Service<
            Request<String>,
            Response = Result<Response<Incoming>, Error>,
            Error = Infallible,
            Future = RevProxyFuture,
        >,
    {
        let _mk = mockito::mock("POST", "/goo")
            .match_body("test")
            .with_body("ok")
            .create();

        call(svc, ("POST", "/foo", None, "test"), (StatusCode::OK, "ok")).await;

        call(
            svc,
            ("PUT", "/foo", None, "test"),
            (StatusCode::NOT_IMPLEMENTED, ""),
        )
        .await;

        call(
            svc,
            ("POST", "/foo", None, "tests"),
            (StatusCode::NOT_IMPLEMENTED, ""),
        )
        .await;
    }

    pub async fn match_header<S>(svc: &mut S)
    where
        S: Service<
            Request<String>,
            Response = Result<Response<Incoming>, Error>,
            Error = Infallible,
            Future = RevProxyFuture,
        >,
    {
        let _mk = mockito::mock("POST", "/goo")
            .match_header("content-type", "application/json")
            .match_body(r#"{"key":"value"}"#)
            .with_body("ok")
            .create();

        call(
            svc,
            (
                "POST",
                "/foo",
                Some("application/json"),
                r#"{"key":"value"}"#,
            ),
            (StatusCode::OK, "ok"),
        )
        .await;

        call(
            svc,
            ("POST", "/foo", None, r#"{"key":"value"}"#),
            (StatusCode::NOT_IMPLEMENTED, ""),
        )
        .await;

        call(
            svc,
            (
                "POST",
                "/foo",
                Some("application/json"),
                r#"{"key":"values"}"#,
            ),
            (StatusCode::NOT_IMPLEMENTED, ""),
        )
        .await;
    }
}
