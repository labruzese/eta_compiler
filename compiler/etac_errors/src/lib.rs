//! Compiler diagnostics.
//!
//! The pipeline is: build a [`Diagnostic`] (plain data), then route it through a
//! [`DiagCtxt`] — the single context that owns an [`Emitter`] and the error/warning
//! counts. Nothing emits except through that context, and an emitted error yields an
//! [`ErrorGuaranteed`] proof token so "we failed" can be made to entail "the user was
//! told why" at the type level.
//!
//! * Plain-data producers (the lexer's logos callbacks, lalrpop's recovered errors) keep
//!   constructing [`Diagnostic`]s directly — often via the [`error!`] macro — because they
//!   have no context on hand. The layer above funnels them in with [`DiagCtxt::emit`].
//! * Code that holds a `&DiagCtxt` should prefer the builders ([`DiagCtxt::err`] etc.),
//!   which return a must-use [`Diag`] that emits or cancels before it drops.

use ariadne::Color;

use etac_span::Span;

#[macro_export]
/// Creates a new Level::Error Diagnostic with a provided message.
/// Note the syntax is to have a semicolon (`;`) after the span.
/// `error!(span; "no identifier called {}", id)` => Diagnostic with span
/// `error!("file does not exist")` => Diagnostic *without* a span
macro_rules! error {
    ($span:expr; $($arg:tt)*) => {
        $crate::Diagnostic::new($crate::Level::Error, $span, format!($($arg)*))
    };
    ($($arg:tt)*) => {
        $crate::Diagnostic::new_no_loc($crate::Level::Error, format!($($arg)*))
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Denotes the severity of the Diagnostic
pub enum Level {
    Error,
    Warning,
    Note,
}

mod dcx;
mod diagnostic;
mod emitter;

pub use dcx::*;
pub use diagnostic::*;
pub use emitter::*;
