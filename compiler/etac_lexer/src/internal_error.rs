use etac_cache::Span;

macro_rules! lexer_error {
    (span = $span:expr, message = $message:expr $(, $key:ident = $val:expr)* $(,)?) => {{
        #[allow(unused_mut)]
        let mut err = $crate::InternalLexerError {
            span: $span,
            message: ::std::string::ToString::to_string(&$message),
            plabel: None,
            note: None,
        };
        $(
            $crate::lexer_error!(@set err, $key, $val);
        )*
        err
    }};

    (@set $err:ident, plabel, $val:expr) => {
        $err.plabel = ::std::option::Option::Some(::std::string::ToString::to_string(&$val));
    };
    (@set $err:ident, note, $val:expr) => {
        $err.note = ::std::option::Option::Some(::std::string::ToString::to_string(&$val));
    };
    (@set $err:ident, $unknown:ident, $val:expr) => {
        compile_error!(concat!("unknown lexer_err field: ", stringify!($unknown)));
    };
}

pub(crate) use lexer_error;

/// Internal lexer error type.
///
/// This is `pub` only because `logos`'s generated `Logos` impl for `Token`
/// requires the associated error type to be at least as visible as `Token`
/// itself (rustc E0446). It is **not** part of this crate's public API,
/// carries no semver guarantees, and should not be named or matched on by
/// downstream crates.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct InternalLexerError {
    pub(crate) span: Span,
    pub(crate) message: String,
    pub(crate) plabel: Option<String>,
    pub(crate) note: Option<String>,
}

impl Default for InternalLexerError {
    fn default() -> Self {
        panic!("Lexer should never use the default error")
    }
}
