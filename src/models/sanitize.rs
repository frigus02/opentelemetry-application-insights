use serde::Serialize;
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Serialize)]
pub(crate) struct LimitedLenString<const N: usize>(String);

impl<const N: usize> From<&str> for LimitedLenString<N> {
    fn from(s: &str) -> Self {
        Self(String::from(&s[0..std::cmp::min(s.len(), N)]))
    }
}

impl<const N: usize> From<String> for LimitedLenString<N> {
    fn from(mut s: String) -> Self {
        s.truncate(N);
        Self(s)
    }
}

impl<'a, const N: usize> From<Cow<'a, str>> for LimitedLenString<N> {
    fn from(s: std::borrow::Cow<'a, str>) -> Self {
        match s {
            Cow::Borrowed(b) => b.into(),
            Cow::Owned(o) => o.into(),
        }
    }
}

impl<const N: usize> From<&opentelemetry::Key> for LimitedLenString<N> {
    fn from(k: &opentelemetry::Key) -> Self {
        k.as_str().into()
    }
}

impl<const N: usize> From<&opentelemetry::Value> for LimitedLenString<N> {
    fn from(v: &opentelemetry::Value) -> Self {
        v.as_str().into()
    }
}

impl<const N: usize> AsRef<str> for LimitedLenString<N> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

pub(crate) type Properties = BTreeMap<LimitedLenString<150>, LimitedLenString<8192>>;
