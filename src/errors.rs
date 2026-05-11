#![allow(unused)]

use crate::sources::{span::EtaSpan, FileId};
use std::{fmt::Debug, ops::Range, rc::Rc};
use std::convert::Infallible;

use ariadne::{Color, Label, Report, ReportKind};

use crate::sources::{SourceId, Sources};

#[macro_export]
macro_rules! error {
    ($name:expr, $span:expr, $($arg:tt)*) => {
        crate::errors::Diagnostic::error(($name, $span).into(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warn {
    ($name:expr, $span:expr, $($arg:tt)*) => {
        Diagnostic::warning(($name, $span).into(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! note {
    ($name:expr, $span:expr, $($arg:tt)*) => {
        Diagnostic::note(($name, $span).into(), format!($($arg)*))
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

/// So that ariadne can report errors given EtaSpans
impl ariadne::Span for EtaSpan {
    type SourceId = SourceId;
    fn source(&self) -> &SourceId { &self.file_id }
    fn start(&self) -> usize   { self.range.start }
    fn end(&self)   -> usize   { self.range.end }
}

/// write the diagnostic to stderr (pretty)
pub fn emit(sources: &mut Sources, diag: Diagnostic) {
    let kind = match diag.level {
        Level::Error   => ReportKind::Error,
        Level::Warning => ReportKind::Warning,
        Level::Note    => ReportKind::Advice,
    };
    let mut b = Report::build(kind, diag.loc).with_message(diag.message);
    if let Some(c) = diag.code { b = b.with_code(c); }
    if let Some(n) = diag.note { b = b.with_note(n); }
    for (span, msg, color) in diag.labels {
        b = b.with_label(Label::new(span).with_message(msg).with_color(color));
    }
    let _ = b.finish().eprint(sources);
}

/// write a diagnostic that doesn't have any locatoin information
pub fn emit_raw(level: crate::errors::Level, msg: impl ToString) {
    let kind = match level {
        Level::Error   => ReportKind::Error,
        Level::Warning => ReportKind::Warning,
        Level::Note    => ReportKind::Advice,
    };
    static NO_SPAN: NoSpan = NoSpan {};
    static NO_CACHE: NoCache = NoCache {};
    Report::build(kind, NO_SPAN)
        .with_message(msg.to_string())
        .finish()
        .eprint(NO_CACHE);
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
