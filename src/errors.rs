#![allow(unused)]

use std::{fmt::Debug, ops::Range, rc::Rc};

use ariadne::{Color, Label, Report, ReportKind};

use crate::sources::{EtaSpan, FileId, Sources};

#[macro_export]
macro_rules! error {
    ($span:expr, $($arg:tt)*) => {
        NoFileDiagnostic::error($span, format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warn {
    ($span:expr, $($arg:tt)*) => {
        NoFileDiagnostic::warning($span, format!($($arg)*))
    };
}

#[macro_export]
macro_rules! note {
    ($span:expr, $($arg:tt)*) => {
        NoFileDiagnostic::note($span, format!($($arg)*))
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

/// Extension trait so diagnostics render via `sources.emit(diag)`.
/// (Inherent impls on foreign type aliases aren't allowed; a trait is.)
pub trait Emit {
    fn emit(&mut self, diag: Diagnostic);
}

impl Emit for Sources {
    fn emit(&mut self, diag: Diagnostic) {
        let kind = match diag.level {
            Level::Error   => ReportKind::Error,
            Level::Warning => ReportKind::Warning,
            Level::Note    => ReportKind::Advice,
        };

        let mut builder = Report::build(kind, diag.loc).with_message(diag.message);

        if let Some(code) = diag.code { builder = builder.with_code(code); }
        if let Some(note) = diag.note { builder = builder.with_note(note); }
        for (span, msg, color) in diag.labels {
            builder = builder.with_label(Label::new(span).with_message(msg).with_color(color));
        }

        let _ = builder.finish().eprint(self);
    }
}
