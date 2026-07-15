//! Diagnostic sinks.
//!
//! An [`Emitter`] is the *only* thing that turns a [`Diagnostic`] into output. The
//! [`DiagCtxt`](crate::DiagCtxt) owns one and routes every diagnostic through it

use std::{cell::RefCell, convert::Infallible, io::Write, rc::Rc};

use ariadne::{Config, IndexType, Label, Report, ReportKind};
use etac_cache::Span;

use crate::{Diag, Level};

/// Can take ownership of a diagnostic to emit it
pub trait Emitter {
    fn emit(&mut self, diag: Diag<'_>);
}

/// Renders diagnostics to stderr with source snippets via `ariadne`.
#[derive(Debug, Default, Clone, Copy)]
pub struct IoEmitter<W: Write> {
    writer: W
}

impl<W: Write> IoEmitter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: Write> Emitter for IoEmitter<W> {
    fn emit(&mut self, diag: Diag<'_>) {
        let kind = match diag.level {
            Level::Error => ReportKind::Error,
            Level::Warning => ReportKind::Warning,
            Level::Note => ReportKind::Advice,
        };

        if let Some(loc) = diag.loc {
            // resolve() returns byte ranges; tell ariadne to interpret span
            // offsets as byte offsets so its line:col header matches what
            // lc_index() (used by the logger) reports.  Without this,
            // ariadne defaults to IndexType::Char and misreports columns
            // whenever multibyte UTF-8 characters appear before the error
            // in the same file.
            let byte_config = Config::default().with_index_type(IndexType::Byte);
            let mut b = Report::build(kind, diag.dcx.cache().reportable_span(loc))
                .with_config(byte_config)
                .with_message(&diag.message);
            if let Some(c) = &diag.code {
                b = b.with_code(c);
            }
            if let Some(n) = &diag.note {
                b = b.with_note(n);
            }
            for (span, msg, color) in &diag.labels {
                b = b.with_label(Label::new(diag.dcx.cache().reportable_span(*span)).with_message(msg).with_color(*color));
            }
            let _ = b.finish().write(diag.dcx.cache(), &mut self.writer);
        } else {
            static NO_SPAN: NoSpan = NoSpan;
            let mut b = Report::build(kind, NO_SPAN).with_message(&diag.message);
            if let Some(c) = &diag.code {
                b = b.with_code(c);
            }
            if let Some(n) = &diag.note {
                b = b.with_note(n);
            }
            for (_span, msg, color) in &diag.labels {
                b = b.with_label(Label::new(NO_SPAN).with_message(msg).with_color(*color));
            }
            let _ = b.finish().write(NoCache, &mut self.writer);
        }
    }
}

/// Records diagnostics into a shared buffer instead of printing them.
///
/// Cloning shares the same underlying buffer (it is an `Rc<RefCell<_>>`), so a test can
/// keep a handle, hand a clone to the [`DiagCtxt`](crate::DiagCtxt), run a phase, and
/// then read back exactly what was emitted via [`take`](BufferEmitter::take).
#[derive(Debug, Clone, Default)]
pub struct BufferEmitter(Rc<RefCell<Vec<RecordedDiag>>>);

impl BufferEmitter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain everything emitted so far, leaving the buffer empty.
    #[must_use]
    pub fn take(&self) -> Vec<RecordedDiag> {
        std::mem::take(&mut self.0.borrow_mut())
    }

    /// Number of diagnostics currently buffered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
}

/// Stored Diag for emitters that are claiming ownership of a Diag
#[non_exhaustive]
#[derive(Debug)]
pub struct RecordedDiag {
    pub level: Level,
    pub message: String,
    pub loc: Option<Span>,
    pub labels: Vec<(Span, String, ariadne::Color)>,
    pub code: Option<String>,
    pub note: Option<String>,
}

impl Emitter for BufferEmitter {
    fn emit(&mut self, diag: Diag<'_>) {
        let rd = RecordedDiag {
            level: diag.level,
            message: diag.message,
            loc: diag.loc,
            labels: diag.labels,
            code: diag.code,
            note: diag.note,
        };
        self.0.borrow_mut().push(rd);
    }
}

// --- ariadne plumbing for the location-less path ---

/// Zero-sized [`ariadne::Span`] for diagnostics that have no source location.
/// All of its impls are no-ops.
#[derive(Clone, Copy)]
struct NoSpan;

/// Zero-sized [`ariadne::Cache`] paired with [`NoSpan`].
#[derive(Clone, Copy)]
struct NoCache;

impl ariadne::Span for NoSpan {
    type SourceId = ();
    fn source(&self) -> &Self::SourceId {
        &()
    }
    fn start(&self) -> usize {
        0
    }
    fn end(&self) -> usize {
        0
    }
}

impl ariadne::Cache<()> for NoCache {
    type Storage = &'static str;
    fn fetch(&mut self, _id: &()) -> Result<&ariadne::Source<Self::Storage>, impl std::fmt::Debug> {
        static SOURCE: std::sync::LazyLock<ariadne::Source<&'static str>> =
            std::sync::LazyLock::new(|| ariadne::Source::from(""));
        Ok::<_, Infallible>(&SOURCE)
    }
    fn display<'a>(&self, _id: &'a ()) -> Option<impl std::fmt::Display + 'a> {
        Some("")
    }
}
