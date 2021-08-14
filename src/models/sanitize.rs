use std::collections::BTreeMap;

macro_rules! limited_len_string {
    ($name:ident, $len:expr) => {
        #[derive(Debug, Eq, PartialEq, PartialOrd, Ord, serde::Serialize)]
        pub(crate) struct $name(String);

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(String::from(&s[0..std::cmp::min(s.len(), $len)]))
            }
        }

        impl From<String> for $name {
            fn from(mut s: String) -> Self {
                s.truncate($len);
                Self(s)
            }
        }

        impl<'a> From<std::borrow::Cow<'a, str>> for $name {
            fn from(s: std::borrow::Cow<'a, str>) -> Self {
                Self(String::from(&s[0..std::cmp::min(s.len(), $len)]))
            }
        }

        impl From<&opentelemetry::Value> for $name {
            fn from(v: &opentelemetry::Value) -> Self {
                v.as_str().into_owned().into()
            }
        }

        impl AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                self.0.as_ref()
            }
        }
    };
}

limited_len_string!(LimitedLenString32768, 32768);
limited_len_string!(LimitedLenString8192, 8192);
limited_len_string!(LimitedLenString2048, 2048);
limited_len_string!(LimitedLenString1024, 1024);
limited_len_string!(LimitedLenString256, 256);
limited_len_string!(LimitedLenString150, 150);
limited_len_string!(LimitedLenString128, 128);
limited_len_string!(LimitedLenString64, 64);
limited_len_string!(LimitedLenString40, 40);

pub(crate) type Properties = BTreeMap<LimitedLenString150, LimitedLenString8192>;
