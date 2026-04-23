#![allow(unused)]

use crate::sources::span::EtaSpan;
use std::{fmt::Debug, ops::Range, rc::Rc};

use ariadne::{Color, Label, Report, ReportKind};

use crate::sources::{FileId, Sources};

#[macro_export]
macro_rules! error {
    ($name:expr, $span:expr, $($arg:tt)*) => {
        Diagnostic::error(($name, $span).into(), format!($($arg)*))
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

mod no_file_diagnostic;
pub use no_file_diagnostic::*;

mod diagnostic;
pub use diagnostic::*;

impl ariadne::Span for EtaSpan {
    type SourceId = FileId;
    fn source(&self) -> &FileId { &self.file_id }
    fn start(&self) -> usize   { self.range.start }
    fn end(&self)   -> usize   { self.range.end }
}

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
