//! A [`PathRewriter`] instance defines a rule to rewrite the request path.
//!
//! A "path" does not include a query. See [`http::uri::Uri`].

use std::borrow::Cow;

use http::uri::{Authority, Scheme, Uri};
use http::Error as HttpError;
use http::Request;

use regex::{Regex as LibRegex, Replacer};

/// Represents a rule to rewrite a path `/foo/bar/baz` to new one.
///
/// A "path" does not include a query. See [`http::uri::Uri`].
pub trait PathRewriter {
    fn rewrite<'a>(&'a mut self, path: &'a str) -> Cow<'a, str>;

    fn rewrite_uri<B>(
        &mut self,
        req: &mut Request<B>,
        scheme: &Scheme,
        authority: &Authority,
    ) -> Result<(), HttpError> {
        let uri = {
            let uri = req.uri();
            let path = self.rewrite(uri.path());
            if let Some(query) = uri.query() {
                let mut p_and_q = path.into_owned();
                p_and_q.push('?');
                p_and_q.push_str(query);

                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(authority.clone())
                    .path_and_query(p_and_q)
                    .build()
            } else {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(authority.clone())
                    .path_and_query(&*path)
                    .build()
            }
        }?;
        *req.uri_mut() = uri;
        Ok(())
    }
}

/// Identity function, that is, this returns the `path` as is.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, Identity};
/// assert_eq!(Identity.rewrite("foo"), "foo");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity;

impl PathRewriter for Identity {
    #[inline]
    fn rewrite<'a>(&mut self, path: &'a str) -> Cow<'a, str> {
        path.into()
    }
}

/// Returns `self.0` regardless what the `path` is.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, Static};
/// assert_eq!(Static("bar").rewrite("foo"), "bar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Static<'a>(pub &'a str);

impl PathRewriter for Static<'_> {
    #[inline]
    fn rewrite<'a>(&'a mut self, _path: &'a str) -> Cow<'a, str> {
        self.0.into()
    }
}

/// `ReplaceAll(old, new)` replaces all matches `old` with `new`.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, ReplaceAll};
/// assert_eq!(ReplaceAll("foo", "bar").rewrite("foofoo"), "barbar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaceAll<'a>(pub &'a str, pub &'a str);

impl PathRewriter for ReplaceAll<'_> {
    fn rewrite<'a>(&mut self, path: &'a str) -> Cow<'a, str> {
        if path.contains(self.0) {
            path.replace(self.0, self.1).into()
        } else {
            path.into()
        }
    }
}

/// `ReplaceN(old, new, n)` replaces first `n` matches `old` with `new`.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, ReplaceN};
/// assert_eq!(ReplaceN("foo", "bar", 1).rewrite("foofoo"), "barfoo");
/// assert_eq!(ReplaceN("foo", "bar", 3).rewrite("foofoo"), "barbar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaceN<'a>(pub &'a str, pub &'a str, pub usize);

impl PathRewriter for ReplaceN<'_> {
    fn rewrite<'a>(&mut self, path: &'a str) -> Cow<'a, str> {
        if path.contains(self.0) {
            path.replacen(self.0, self.1, self.2).into()
        } else {
            path.into()
        }
    }
}

/// Trims a prefix if exists.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, TrimPrefix};
/// assert_eq!(TrimPrefix("foo").rewrite("foobarfoo"), "barfoo");
/// assert_eq!(TrimPrefix("bar").rewrite("foobarfoo"), "foobarfoo");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimPrefix<'a>(pub &'a str);

impl PathRewriter for TrimPrefix<'_> {
    fn rewrite<'a>(&mut self, path: &'a str) -> Cow<'a, str> {
        if let Some(stripped) = path.strip_prefix(self.0) {
            stripped.into()
        } else {
            path.into()
        }
    }
}

/// Trims a suffix if exists.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, TrimSuffix};
/// assert_eq!(TrimSuffix("foo").rewrite("foobarfoo"), "foobar");
/// assert_eq!(TrimSuffix("bar").rewrite("foobarfoo"), "foobarfoo");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimSuffix<'a>(pub &'a str);

impl PathRewriter for TrimSuffix<'_> {
    fn rewrite<'a>(&mut self, path: &'a str) -> Cow<'a, str> {
        if let Some(stripped) = path.strip_suffix(self.0) {
            stripped.into()
        } else {
            path.into()
        }
    }
}

/// Appends a prefix.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, AppendPrefix};
/// assert_eq!(AppendPrefix("foo").rewrite("bar"), "foobar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendPrefix<'a>(pub &'a str);

impl PathRewriter for AppendPrefix<'_> {
    fn rewrite<'a>(&mut self, path: &'a str) -> Cow<'a, str> {
        let mut ret = String::with_capacity(self.0.len() + path.len());
        ret.push_str(self.0);
        ret.push_str(path);
        ret.into()
    }
}

/// Appends a suffix.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, AppendSuffix};
/// assert_eq!(AppendSuffix("foo").rewrite("bar"), "barfoo");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendSuffix<'a>(pub &'a str);

impl PathRewriter for AppendSuffix<'_> {
    fn rewrite<'a>(&mut self, path: &'a str) -> Cow<'a, str> {
        let mut ret = String::with_capacity(self.0.len() + path.len());
        ret.push_str(path);
        ret.push_str(self.0);
        ret.into()
    }
}

/// `RegexAll(re, new)` replaces all matches `re` with `new`.
///
/// The type of `new` must implement [`Replacer`].
/// See [`regex`] for details.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, RegexAll};
/// # use regex::Regex;
/// let re = Regex::new(r"(?P<y>\d{4})/(?P<m>\d{2})").unwrap();
/// assert_eq!(
///     RegexAll(re, "$m-$y").rewrite("2021/10/2022/12"),
///     "10-2021/12-2022"
/// );
/// ```
#[derive(Debug, Clone)]
pub struct RegexAll<Rep>(pub LibRegex, pub Rep);

impl<Rep: Replacer> PathRewriter for RegexAll<Rep> {
    fn rewrite<'a>(&mut self, path: &'a str) -> Cow<'a, str> {
        self.0.replace_all(path, self.1.by_ref())
    }
}

/// `RegexN(re, new, n)` replaces first `n` matches `re` with `new`.
///
/// The type of `new` must implement [`Replacer`].
/// See [`regex`] for details.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, RegexN};
/// # use regex::Regex;
/// let re = Regex::new(r"(?P<y>\d{4})/(?P<m>\d{2})").unwrap();
/// assert_eq!(
///     RegexN(re.clone(), "$m-$y", 1).rewrite("2021/10/2022/12"),
///     "10-2021/2022/12"
/// );
/// assert_eq!(
///     RegexN(re, "$m-$y", 3).rewrite("2021/10/2022/12"),
///     "10-2021/12-2022"
/// );
/// ```
#[derive(Debug, Clone)]
pub struct RegexN<Rep>(pub LibRegex, pub Rep, pub usize);

impl<Rep: Replacer> PathRewriter for RegexN<Rep> {
    fn rewrite<'a>(&mut self, path: &'a str) -> Cow<'a, str> {
        self.0.replacen(path, self.2, self.1.by_ref())
    }
}

/// Converts the `path` by a function.
///
/// The type of the function must be `for<'a> FnMut(&'a str) -> String`.
///
/// ```
/// # use reverse_proxy_service::rewrite::{PathRewriter, Func};
/// let f = |path: &str| path.len().to_string();
/// assert_eq!(Func(f).rewrite("abc"), "3");
/// ```
pub struct Func<F>(pub F);

impl<F> PathRewriter for Func<F>
where
    for<'a> F: FnMut(&'a str) -> String,
{
    fn rewrite<'a>(&'a mut self, path: &'a str) -> Cow<'a, str> {
        self.0(path).into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rewrite_static() {
        let path = "/foo/bar";
        let mut rw = Static("/baz");
        assert_eq!(rw.rewrite(path), "/baz");
    }

    #[test]
    fn replace() {
        let path = "/foo/bar/foo/baz/foo";
        let mut rw = ReplaceAll("foo", "FOO");
        assert_eq!(rw.rewrite(path), "/FOO/bar/FOO/baz/FOO");

        let path = "/foo/bar/foo/baz/foo";
        let mut rw = ReplaceAll("/foo", "");
        assert_eq!(rw.rewrite(path), "/bar/baz");

        let path = "/foo/bar/foo/baz/foo";
        let mut rw = ReplaceN("foo", "FOO", 2);
        assert_eq!(rw.rewrite(path), "/FOO/bar/FOO/baz/foo");
    }

    #[test]
    fn trim() {
        let path = "/foo/foo/bar";
        let mut rw = TrimPrefix("/foo");
        assert_eq!(rw.rewrite(path), "/foo/bar");

        let path = "/foo/foo/bar";
        let mut rw = TrimPrefix("foo");
        assert_eq!(rw.rewrite(path), "/foo/foo/bar");

        let path = "/bar/foo/foo";
        let mut rw = TrimSuffix("foo");
        assert_eq!(rw.rewrite(path), "/bar/foo/");

        let path = "/bar/foo/foo";
        let mut rw = TrimSuffix("foo/");
        assert_eq!(rw.rewrite(path), "/bar/foo/foo");
    }

    #[test]
    fn append() {
        let path = "/foo/bar";
        let mut rw = AppendPrefix("/baz");
        assert_eq!(rw.rewrite(path), "/baz/foo/bar");

        let path = "/foo/bar";
        let mut rw = AppendSuffix("/baz");
        assert_eq!(rw.rewrite(path), "/foo/bar/baz");
    }

    #[test]
    fn regex() {
        let path = "/2021/10/21/2021/12/02/2022/01/13";
        let mut rw = RegexAll(
            LibRegex::new(r"(?P<y>\d{4})/(?P<m>\d{2})/(?P<d>\d{2})").unwrap(),
            "$m-$d-$y",
        );
        assert_eq!(rw.rewrite(path), "/10-21-2021/12-02-2021/01-13-2022");

        let path = "/2021/10/21/2021/12/02/2022/01/13";
        let mut rw = RegexN(
            LibRegex::new(r"(?P<y>\d{4})/(?P<m>\d{2})/(?P<d>\d{2})").unwrap(),
            "$m-$d-$y",
            2,
        );
        assert_eq!(rw.rewrite(path), "/10-21-2021/12-02-2021/2022/01/13");
    }

    #[test]
    fn func() {
        let path = "/abcdefg";
        let mut rw = Func(|path: &str| path.len().to_string());
        assert_eq!(rw.rewrite(path), "8");
    }
}
