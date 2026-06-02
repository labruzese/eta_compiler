#![allow(unused)]

use std::{fmt::Debug, ops::Range, rc::Rc};
use std::convert::Infallible;
use ariadne::{Color, Label, Report, ReportKind};

use etac_span::{Span, FileId, SourceId, SourceCache};

#[macro_export]
macro_rules! error {
    ($span:expr; $($arg:tt)*) => {
        $crate::Diagnostic::new($crate::Level::Error, $span, format!($($arg)*))
    };
    ($($arg:tt)*) => {
        $crate::Diagnostic::new_no_loc($crate::Level::Error, format!($($arg)*))
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Note,
}

mod diagnostic;
pub use diagnostic::*;

/// write the diagnostic to stderr (pretty)
pub fn emit(source_cache: &mut SourceCache, diag: Diagnostic) {
    let kind = match diag.level {
        Level::Error   => ReportKind::Error,
        Level::Warning => ReportKind::Warning,
        Level::Note    => ReportKind::Advice,
    };
    static NO_SPAN: NoSpan = NoSpan {};
    static NO_CACHE: NoCache = NoCache {};


    match diag.loc {
        Some(loc) => {
            let floc = source_cache.resolve(loc);
            let mut b = Report::build(kind, floc)
                .with_message(diag.message);
            if let Some(c) = diag.code { b = b.with_code(c); }
            if let Some(n) = diag.note { b = b.with_code(n); }
            for (span, msg, color) in diag.labels {
                let fspan = source_cache.resolve(span);
                b = b.with_label(Label::new(fspan).with_message(msg).with_color(color));
            }
            let _ = b.finish().eprint(source_cache);
        },
        None      => {
            let mut b = Report::build(kind, NO_SPAN)
                .with_message(diag.message);
            if let Some(c) = diag.code { b = b.with_code(c); }
            if let Some(n) = diag.note { b = b.with_code(n); }
            for (span, msg, color) in diag.labels {
                b = b.with_label(Label::new(NO_SPAN).with_message(msg).with_color(color));
            }
            let _ = b.finish().eprint(NO_CACHE);
        },
    };
}

#[derive(Clone, Copy)]
/// dummy struct for satisfying ariadne when we don't have a source
pub struct NoSpan {}
#[derive(Clone, Copy)]
/// dummy struct for satisfying ariadne when we don't have a source
pub struct NoCache {}
impl ariadne::Span for NoSpan {
    type SourceId = ();
    fn source(&self) -> &Self::SourceId {&()}
    fn start(&self) -> usize {0}
    fn end(&self) -> usize {0}
}
impl ariadne::Cache<()> for NoCache {
    type Storage = &'static str;
    fn fetch(&mut self, id: &()) -> Result<&ariadne::Source<Self::Storage>, impl std::fmt::Debug> {
        static SOURCE: std::sync::LazyLock<ariadne::Source<&'static str>> 
                     = std::sync::LazyLock::new(||ariadne::Source::from(""));
        Ok::<_, Infallible>(&SOURCE)
    }
    fn display<'a>(&self, id: &'a ()) -> Option<impl std::fmt::Display + 'a> {
        Some("")
    }
}
