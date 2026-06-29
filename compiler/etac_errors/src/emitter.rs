//! Diagnostic sinks.
//!
//! An [`Emitter`] is the *only* thing that turns a [`Diagnostic`] into output. The
//! [`DiagCtxt`](crate::DiagCtxt) owns one and routes every diagnostic through it, so
//! swapping the sink (human-readable stderr, an in-memory buffer for tests, JSON later)
//! is a one-line change and never touches call sites.

use std::cell::RefCell;
use std::convert::Infallible;
use std::rc::Rc;

use ariadne::{Label, Report, ReportKind};
use etac_span::SourceCache;

use crate::{Diagnostic, Level};

/// The single point at which a diagnostic becomes output.
///
/// Takes the diagnostic *by value*: the [`DiagCtxt`](crate::DiagCtxt) reads everything
/// it needs (the level, for counting) before handing it over, so an emitter is free to
/// consume the payload without cloning.
pub trait Emitter {
    fn emit(&mut self, diag: Diagnostic, sources: &SourceCache);
}

/// Renders diagnostics to stderr with source snippets via `ariadne`.
#[derive(Debug, Default, Clone, Copy)]
pub struct HumanEmitter;

impl Emitter for HumanEmitter {
    fn emit(&mut self, diag: Diagnostic, sources: &SourceCache) {
        let kind = match diag.level {
            Level::Error => ReportKind::Error,
            Level::Warning => ReportKind::Warning,
            Level::Note => ReportKind::Advice,
        };

        match diag.loc {
            Some(loc) => {
                let floc = sources.resolve(loc);
                let mut b = Report::build(kind, floc).with_message(diag.message);
                if let Some(c) = diag.code {
                    b = b.with_code(c);
                }
                if let Some(n) = diag.note {
                    // (was `with_code` here before — a note was being rendered as a code.)
                    b = b.with_note(n);
                }
                for (span, msg, color) in diag.labels {
                    let fspan = sources.resolve(span);
                    b = b.with_label(Label::new(fspan).with_message(msg).with_color(color));
                }
                // `cache_view()` borrows `sources` immutably; see SourceCache::cache_view.
                let _ = b.finish().eprint(sources.cache_view());
            }
            None => {
                static NO_SPAN: NoSpan = NoSpan;
                let mut b = Report::build(kind, NO_SPAN).with_message(diag.message);
                if let Some(c) = diag.code {
                    b = b.with_code(c);
                }
                if let Some(n) = diag.note {
                    b = b.with_note(n);
                }
                for (_span, msg, color) in diag.labels {
                    b = b.with_label(Label::new(NO_SPAN).with_message(msg).with_color(color));
                }
                let _ = b.finish().eprint(NoCache);
            }
        }
    }
}

/// Records diagnostics into a shared buffer instead of printing them.
///
/// Cloning shares the same underlying buffer (it is an `Rc<RefCell<_>>`), so a test can
/// keep a handle, hand a clone to the [`DiagCtxt`](crate::DiagCtxt), run a phase, and
/// then read back exactly what was emitted via [`take`](BufferEmitter::take).
#[derive(Debug, Clone, Default)]
pub struct BufferEmitter(Rc<RefCell<Vec<Diagnostic>>>);

impl BufferEmitter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain everything emitted so far, leaving the buffer empty.
    pub fn take(&self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.0.borrow_mut())
    }

    /// Number of diagnostics currently buffered.
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
}

impl Emitter for BufferEmitter {
    fn emit(&mut self, diag: Diagnostic, _sources: &SourceCache) {
        self.0.borrow_mut().push(diag);
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
