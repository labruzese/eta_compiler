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
use crate::{Diagnostic, Level};

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
    pub fn claim_already_emitted() -> Self {
        ErrorGuaranteed(())
    }
}

struct Inner<'a> {
    emitter: Box<dyn Emitter + 'a>,
    err_count: usize,
    warn_count: usize,
}

/// The single diagnostic sink for a compilation. Borrows the [`SourceCache`] so it can
/// render spans; shares it with the rest of the compiler (the cache is interior-mutable).
pub struct DiagCtxt<'a> {
    sources: &'a SourceCache,
    inner: RefCell<Inner<'a>>,
}

impl<'a> DiagCtxt<'a> {
    /// A context that renders to stderr.
    pub fn new(sources: &'a SourceCache) -> Self {
        Self::with_emitter(sources, Box::new(HumanEmitter))
    }

    /// A context with a custom sink (e.g. [`BufferEmitter`](crate::BufferEmitter) in tests).
    pub fn with_emitter(sources: &'a SourceCache, emitter: Box<dyn Emitter + 'a>) -> Self {
        Self {
            sources,
            inner: RefCell::new(Inner { emitter, err_count: 0, warn_count: 0 }),
        }
    }

    /// The source cache this context renders against.
    #[inline]
    pub fn sources(&self) -> &'a SourceCache {
        self.sources
    }

    /// Emit a fully-built [`Diagnostic`]. Returns proof iff it was an error.
    ///
    /// This is the funnel for diagnostics produced as plain data — the lexer's logos
    /// callbacks and lalrpop's recovered errors, which have no `DiagCtxt` on hand when
    /// they are constructed. Code that *does* have the context should prefer the
    /// builders ([`err`](Self::err) etc.) for the drop-bomb guarantee.
    pub fn emit(&self, diag: Diagnostic) -> Option<ErrorGuaranteed> {
        let level = diag.level;
        self.inner.borrow_mut().emitter.emit(diag, self.sources);
        let mut inner = self.inner.borrow_mut();
        match level {
            Level::Error => {
                inner.err_count += 1;
                Some(ErrorGuaranteed::new())
            }
            Level::Warning => {
                inner.warn_count += 1;
                None
            }
            Level::Note => None,
        }
    }

    /// Start building an error at `span`. Must be `.emit()`ed or `.cancel()`ed.
    pub fn err(&self, span: Span, msg: impl Into<String>) -> Diag<'_, 'a> {
        Diag::new(self, Diagnostic::new(Level::Error, span, msg))
    }

    /// Start building a location-less error (I/O failures, bad CLI input, …).
    pub fn err_no_span(&self, msg: impl Into<String>) -> Diag<'_, 'a> {
        Diag::new(self, Diagnostic::new_no_loc(Level::Error, msg))
    }

    /// Start building a warning at `span`.
    pub fn warn(&self, span: Span, msg: impl Into<String>) -> Diag<'_, 'a> {
        Diag::new(self, Diagnostic::new(Level::Warning, span, msg))
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
/// Two lifetimes: `'dcx` is how long this builder borrows the context, `'src` is the
/// context's own borrow of the [`SourceCache`]. Keeping them separate is what lets the
/// context be an ordinary local (`let dcx = DiagCtxt::new(&cache);`) — a single shared
/// lifetime here would force the borrow of `dcx` to last as long as the cache borrow.
#[must_use = "a Diag does nothing until you call `.emit()` (or `.cancel()` it)"]
pub struct Diag<'dcx, 'src> {
    dcx: &'dcx DiagCtxt<'src>,
    /// `None` once consumed by `emit`/`cancel`; `Some` means "still owes an emit".
    diag: Option<Diagnostic>,
}

impl<'dcx, 'src> Diag<'dcx, 'src> {
    fn new(dcx: &'dcx DiagCtxt<'src>, diag: Diagnostic) -> Self {
        Self { dcx, diag: Some(diag) }
    }

    #[inline]
    fn map(mut self, f: impl FnOnce(Diagnostic) -> Diagnostic) -> Self {
        let d = self.diag.take().expect("Diag already consumed");
        self.diag = Some(f(d));
        self
    }

    /// Point the primary (red) label at the diagnostic's own span.
    pub fn with_primary_label(self, msg: impl Into<String>) -> Self {
        self.map(|d| d.with_primary_label(msg))
    }

    /// Add a secondary (yellow) label at another span.
    pub fn with_secondary_label(self, span: Span, msg: impl Into<String>) -> Self {
        self.map(|d| d.with_secondary_label(span, msg))
    }

    pub fn with_note(self, note: impl Into<String>) -> Self {
        self.map(|d| d.with_note(note))
    }

    pub fn with_code(self, code: impl Into<String>) -> Self {
        self.map(|d| d.with_code(code))
    }

    /// Emit through the context. Returns proof iff this was an error.
    pub fn emit(mut self) -> Option<ErrorGuaranteed> {
        let d = self.diag.take().expect("Diag already consumed");
        self.dcx.emit(d)
    }

    /// Throw the diagnostic away deliberately, defusing the drop-bomb.
    pub fn cancel(mut self) {
        self.diag = None;
    }
}

impl Drop for Diag<'_, '_> {
    fn drop(&mut self) {
        if let Some(diag) = self.diag.take() {
            // A diagnostic was built and then dropped on the floor. In debug that is a
            // bug worth surfacing loudly; in release we still emit it so the user is
            // never silently denied an error they should have seen.
            if cfg!(debug_assertions) && !std::thread::panicking() {
                panic!("Diag dropped without `.emit()`/`.cancel()`: {diag:?}");
            }
            self.dcx.emit(diag);
        }
    }
}

impl fmt::Debug for DiagCtxt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("DiagCtxt")
            .field("err_count", &inner.err_count)
            .field("warn_count", &inner.warn_count)
            .finish_non_exhaustive()
    }
}
