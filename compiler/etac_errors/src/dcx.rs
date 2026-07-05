//! The diagnostic context — the one sink every diagnostic flows through.
//!
//! It has three parts:
//!
//! * [`DiagCtxt`] owns the [`Emitter`] and the running error/warning counts. Nothing
//!   else emits. Phases borrow `&DiagCtxt` and report directly; the driver never
//!   collects a `Vec<Diagnostic>` to drain later.
//!
//! * [`ErrorGuaranteed`] is a zero-sized *proof* that an error reached the user. It can
//!   only be minted on an error path inside this module, so a function returning
//!   `Result<T, ErrorGuaranteed>` is making a type-level promise: "if this is `Err`, a
//!   diagnostic was emitted." No more silent failures.
//!
//! * [`Diag`] is a builder bound to the context. It carries a drop-bomb: a diagnostic
//!   that is built but neither `.emit()`ed nor `.cancel()`ed is a bug, caught in debug
//!   and emitted anyway in release so it is never silently lost.

use std::cell::RefCell;
use std::fmt;

use etac_span::{SourceCache, Span};

use crate::emitter::{Emitter, HumanEmitter};
use crate::{Level};
use crate::drop_bomb::DropBomb;

/// Zero-sized proof that a compilation error was reported through a [`DiagCtxt`].
///
/// Construct it only by *actually emitting* an error (or via the clearly-named
/// [`ErrorGuaranteed::claim_already_emitted`] escape hatch). Thread it through
/// `Result<T, ErrorGuaranteed>` to make "we returned `Err`" entail "the user saw why."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorGuaranteed(());

impl ErrorGuaranteed {
    /// Mint a proof. Private so it can only originate on a real error path in this module.
    #[inline]
    pub(crate) fn new() -> Self {
        ErrorGuaranteed(())
    }

    /// Assert that an error was already reported elsewhere, without emitting one here.
    ///
    /// The deliberately loud name is the point: reach for this only when the proof is
    /// genuinely unavailable to thread (e.g. reconstructing one after a `has_errors`
    /// check across an API boundary). Misuse reintroduces exactly the silent-failure
    /// class this type exists to prevent.
    #[inline]
    #[must_use]
    pub fn claim_already_emitted() -> Self {
        ErrorGuaranteed(())
    }
}

pub(crate) struct Inner {
    emitter: Box<dyn Emitter>,
    err_count: usize,
    warn_count: usize,
}

/// The single diagnostic sink for a compilation. Renders spans against the
/// process-global [`SOURCES`](etac_span::SOURCES) cache, so it carries no
/// lifetime parameter and neither does anything built on it.
pub struct DiagCtxt {
    pub(crate) sources: &'static SourceCache,
    pub(crate) inner: RefCell<Inner>,
}

impl DiagCtxt {
    /// A context that renders to stderr.
    #[must_use]
    pub fn new(cache: &'static SourceCache) -> Self {
        Self::with_emitter(cache, Box::new(HumanEmitter))
    }

    /// A context with a custom sink (e.g. [`BufferEmitter`](crate::BufferEmitter) in tests).
    #[must_use]
    pub fn with_emitter(cache: &'static SourceCache, emitter: Box<dyn Emitter>) -> Self {
        Self {
            sources: cache,
            inner: RefCell::new(Inner { emitter, err_count: 0, warn_count: 0 }),
        }
    }

    /// The source cache this context renders against (the process global).
    #[inline]
    pub fn sources(&self) -> &'static SourceCache {
        self.sources
    }

    /// Start building an error at `span`. Must be `.emit()`ed or `.cancel()`ed.
    pub fn err(&self, span: Span, msg: impl Into<String>) -> Diag<'_> {
        Diag::new(self, Level::Error, span, msg)
    }

    /// Start building a location-less error (I/O failures, bad CLI input, …).
    pub fn err_no_span(&self, msg: impl Into<String>) -> Diag<'_> {
        Diag::new_no_span(self, Level::Error, msg)
    }

    /// Start building a warning at `span`.
    pub fn warn(&self, span: Span, msg: impl Into<String>) -> Diag<'_> {
        Diag::new(self, Level::Warning, span, msg)
    }

    pub fn err_count(&self) -> usize {
        self.inner.borrow().err_count
    }

    pub fn warn_count(&self) -> usize {
        self.inner.borrow().warn_count
    }

    /// `Some(proof)` iff at least one error has been emitted. The natural thing for the
    /// driver to check before moving to the next phase.
    pub fn has_errors(&self) -> Option<ErrorGuaranteed> {
        (self.err_count() > 0).then(ErrorGuaranteed::new)
    }
}

/// A diagnostic under construction, bound to its [`DiagCtxt`].
///
/// `#[must_use]` plus the [`Drop`] bomb make "built but never emitted" hard to do by
/// accident. Build, decorate, then finish with [`emit`](Diag::emit) or
/// [`cancel`](Diag::cancel).
///
/// One lifetime: `'dcx` is how long this builder borrows the context. (There used
/// to be a second, `'src`, for the context's borrow of the [`SourceCache`]; the
/// cache is now the process-global [`SOURCES`](etac_span::SOURCES), so that borrow
/// is `'static` and the parameter is gone.)
#[must_use = "a Diag does nothing until you call `.emit()` (or `.cancel()` it)"]
#[derive(Debug)]
pub struct Diag<'dcx> {
    pub(crate) dcx: &'dcx DiagCtxt,
    pub level: Level,
    pub message: String,
    pub loc: Option<Span>,
    pub labels: Vec<(Span, String, ariadne::Color)>,
    pub code: Option<String>,
    pub note: Option<String>,
    bomb: DropBomb,
}

impl<'dcx> Diag<'dcx> {
    /// Create a new diagnostic at a location with a message.
    fn new(dcx: &'dcx DiagCtxt, level: Level, span: Span, message: impl Into<String>) -> Self {
        Self {
            dcx,
            level,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            loc: Some(span),
            note: None,
            bomb: DropBomb::new()
        }
    }

    /// Create a new diagnostic that doesn't have a location
    fn new_no_span(dcx: &'dcx DiagCtxt, level: Level, message: impl Into<String>) -> Self {
        Self {
            dcx,
            level,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            loc: None,
            note: None,
            bomb: DropBomb::new(),
        }
    }

    pub fn io(dcx: &'dcx DiagCtxt, io_err: &std::io::Error) -> Self {
        Self::new_no_span(dcx, Level::Error, io_err.to_string())
    }

    /// Point the primary (red) label at the diagnostic's own span.
    pub fn with_primary_label(mut self, msg: impl Into<String>) -> Self {
        self.labels.push((
            self.loc
                .unwrap_or_else(|| panic!("can not add primary label to a diagnostic without a location")),
            msg.into(),
            ariadne::Color::Red,
        ));
        self
    }

    /// Add a secondary (yellow) label at another span.
    pub fn with_secondary_label(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.labels.push((span, msg.into(), ariadne::Color::Yellow));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Emit a fully-built [`Diagnostic`]. Returns proof iff it was an error.
    ///
    /// This is the funnel for diagnostics produced as plain data — the lexer's logos
    /// callbacks and lalrpop's recovered errors, which have no `DiagCtxt` on hand when
    /// they are constructed. Code that *does* have the context should prefer the
    /// builders ([`err`](Self::err) etc.) for the drop-bomb guarantee.
    pub fn emit(mut self) -> ErrorGuaranteed {
        let level = self.level;
        let mut inner = self.dcx.inner.borrow_mut();
        match level {
            Level::Error => {
                inner.err_count += 1;
            }
            Level::Warning => {
                inner.warn_count += 1;
            }
            _ => ()
        }
        self.bomb.defuse();
        inner.emitter.emit(self);
        ErrorGuaranteed::new()
    }

    /// Throw the diagnostic away deliberately, defusing the drop-bomb.
    pub fn cancel(mut self) { self.bomb.defuse() }
}

impl Default for DiagCtxt {
    fn default() -> Self {
        Self::new(etac_span::sources())
    }
}

impl fmt::Debug for DiagCtxt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("DiagCtxt")
            .field("err_count", &inner.err_count)
            .field("warn_count", &inner.warn_count)
            .finish_non_exhaustive()
    }
}
