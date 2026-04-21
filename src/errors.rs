#![allow(unused)]

use std::{fmt::Debug, ops::Range};

use ariadne::{Color, Label, Report, ReportKind};

use crate::sources::{EtaSpan, FileId, SourceManager};

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

/// Interface rest of project with ariadne
impl<'fid> ariadne::Span for EtaSpan<'fid> {
    type SourceId = &'fid FileId;

    fn source(&self) -> &Self::SourceId {
        &self.file_id
    }

    fn start(&self) -> usize {
        self.range.start
    }

    fn end(&self) -> usize {
        self.range.end
    }
}

impl SourceManager {
    /// the errors module gives SourceManager the ability to emit a diagnostic at a span
    pub fn emit(&self, diag: Diagnostic, source: &FileId) {
        let fid = source;

        let kind = match diag.level {
            Level::Error => ReportKind::Error,
            Level::Warning => ReportKind::Warning,
            Level::Note => ReportKind::Advice,
        };

        let mut builder = Report::build(kind, diag.loc).with_message(diag.message);

        if let Some(code) = diag.code {
            builder = builder.with_code(code);
        }

        if let Some(note) = diag.note {
            builder = builder.with_note(note);
        }

        for (span, label_msg, color) in diag.labels {
            builder =
                builder.with_label(Label::new(span).with_message(label_msg).with_color(color));
        }

        // Print to stderr
        if let Some(src) = self.get_source_str(&fid) {
            let _ = builder.finish().eprint((fid, ariadne::Source::from(src)));
        }
    }
}
