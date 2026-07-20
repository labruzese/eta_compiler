use std::sync::atomic::{AtomicUsize, Ordering};

use etac_cache::sources::{SourceMap, Span};

use crate::Level;
use crate::emitter::{Emitter, IoEmitter};

#[cfg(debug_assertions)]
use crate::drop_bomb::DropBomb;
use crate::guarentee::ErrorGuaranteed;

pub struct DiagCtx {
    diagnostics: elsa::sync::FrozenVec<Box<Diagnostic>>,
    emitter: Box<dyn Emitter>,
    err_count: AtomicUsize,
    warn_count: AtomicUsize,
    #[cfg(debug_assertions)] bomb: DropBomb,
}

impl Default for DiagCtx {
    fn default() -> Self {
        Self::new()
    }
}
impl DiagCtx {
    /// A context that renders to stderr.
    pub fn new() -> Self {
        Self::with_emitter(Box::new(IoEmitter::new(std::io::stderr())))
    }

    /// A context with a custom sink (example: [`BufferEmitter`](crate::BufferEmitter)).
    #[must_use]
    pub fn with_emitter(emitter: Box<dyn Emitter>) -> Self {
        Self {
            diagnostics: elsa::sync::FrozenVec::new(),
            emitter,
            err_count: 0.into(),
            warn_count: 0.into(),
            #[cfg(debug_assertions)] bomb: DropBomb::new("DiagCtx dropped without writing diagnostics to emitter"),
        }
    }

    /// Start building an error at `span`. 
    pub fn err(&self, span: Span, msg: impl Into<String>) -> Diag<'_> {
        Diag::new(self, Level::Error, span, msg)
    }

    /// Start building a location-less error.
    pub fn err_no_span(&self, msg: impl Into<String>) -> Diag<'_> {
        Diag::new_no_span(self, Level::Error, msg)
    }

    /// Start building a warning at `span`.
    pub fn warn(& self, span: Span, msg: impl Into<String>) -> Diag<'_> {
        Diag::new(self, Level::Warning, span, msg)
    }

    pub fn io_err(&self, io_err: std::io::Error) -> Diag<'_> {
        self.err_no_span(io_err.to_string())
    }

    pub fn err_count(&self) -> usize {
        self.err_count.load(Ordering::Acquire)
    }

    pub fn warn_count(&self) -> usize {
        self.warn_count.load(Ordering::Acquire)
    }

    pub fn has_errors(&self) -> bool {
        self.err_count() > 0
    }

    pub fn emit_diagnostics(mut self, source_map: &SourceMap) {
        let mut emit_cache = crate::emitter::EmitCache::new(source_map);
        for diag in self.diagnostics.into_vec().drain(..) {
            self.emitter.emit(&mut emit_cache, *diag);
        }
        #[cfg(debug_assertions)] self.bomb.defuse();
    }

    /// deliberately don't emit the diagnostics to emitter
    pub fn cancel(#[cfg_attr(not(debug_assertions), allow(unused_mut))] mut self) {
        #[cfg(debug_assertions)] self.bomb.defuse();
    }


}

#[derive(Debug)]
pub(crate) struct Diagnostic {
    pub level: Level,
    pub message: String,
    pub loc: Option<Span>,
    pub labels: Vec<(Span, String, ariadne::Color)>,
    pub code: Option<String>,
    pub note: Option<String>,
}

/// A diagnostic under construction, knowing its [`DiagCtxt`].
///
/// [`Drop`] bomb will panic in debug mode if dropped without [`emit`](Diag::emit) or
/// [`cancel`](Diag::cancel).
///
/// The single lifetime is the borrow of the context; `DiagCtxt` is covariant
/// over its cache lifetime, so `&'dcx DiagCtxt<'ec>` shrinks to
/// `&'dcx DiagCtxt<'dcx>` at construction.
#[must_use = "a Diag does nothing until you call `.emit()` (or `.cancel()` it)"]
pub struct Diag<'dcx> {
    pub(crate) dcx: &'dcx DiagCtx,
    pub diagnostic: Box<Diagnostic>,
    #[cfg(debug_assertions)] bomb: DropBomb,
}
impl<'dcx> Diag<'dcx> {
    /// Create a new diagnostic at a location with a message.
    fn new(dcx: &'dcx DiagCtx, level: Level, span: Span, message: impl Into<String>) -> Self {
        Self {
            dcx,
            diagnostic: Box::new(Diagnostic {
                level,
                code: None,
                message: message.into(),
                labels: Vec::new(),
                loc: Some(span),
                note: None,
            }),
            #[cfg(debug_assertions)]
            bomb: DropBomb::new("Diag dropped without writing diagnostic to context"),
        }
    }

    /// Create a new diagnostic that doesn't have a location
    fn new_no_span(dcx: &'dcx DiagCtx, level: Level, message: impl Into<String>) -> Self {
        Self {
            dcx,
            diagnostic: Box::new(Diagnostic {
                level,
                code: None,
                message: message.into(),
                labels: Vec::new(),
                loc: None,
                note: None,
            }),
            #[cfg(debug_assertions)]
            bomb: DropBomb::new("Diag dropped without writing diagnostic to context"),
        }
    }

    /// Point the primary (red) label at the diagnostic's own span.
    pub fn with_primary_label(mut self, msg: impl Into<String>) -> Self {
        self.diagnostic.labels.push((
            self.diagnostic.loc
                .unwrap_or_else(|| panic!("can not add primary label to a diagnostic without a location")),
            msg.into(),
            ariadne::Color::Red,
        ));
        self
    }

    /// Add a secondary (yellow) label at another span.
    pub fn with_secondary_label(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.diagnostic.labels.push((span, msg.into(), ariadne::Color::Yellow));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.diagnostic.note = Some(note.into());
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.diagnostic.code = Some(code.into());
        self
    }

    /// Emit a fully-built diagnostic.
    pub fn finish(mut self) -> ErrorGuaranteed {
        let level = self.diagnostic.level;
        match level {
            Level::Error => {
                self.dcx.err_count.fetch_add(1, Ordering::Release);
            }
            Level::Warning => {
                self.dcx.warn_count.fetch_add(1, Ordering::Release);
            }
            _ => (),
        }

        #[cfg(debug_assertions)]
        self.bomb.defuse();

        self.dcx.diagnostics.push(self.diagnostic);
        ErrorGuaranteed::new()
    }

    /// Throw the diagnostic away deliberately (drop without panic in debug mode)
    pub fn cancel(#[cfg_attr(not(debug_assertions), allow(unused_mut))] mut self) {
        #[cfg(debug_assertions)]
        self.bomb.defuse();
    }
}

// use std::fmt::Write;
// impl Diag<'_> {
//     pub fn test_format(&self, cache: &EtaCache) -> String {
//         let mut out = String::new();
//         let loc = self.loc;
//         let level = &self.level;
//         let message = &self.message;
//         let note = self.note.as_deref().unwrap_or("");
//         let mut labels = String::new();
//         self.labels.iter().for_each(|(span, message, ..)| {
//             let file = cache.source_name(cache.resolve_span(*span).1);
//             let (line_start, column_start) = cache.line_column(span.lo);
//             let (line_end, column_end) = cache.line_column(span.hi);
//             let _ = writeln!(labels, "\n\t{file}:{line_start}:{column_start}..{line_end}:{column_end} {message:?}");
//         });
//         let diag_str = format!("{level:?} {{\n\tmessage: {message}\n\tnote: {note}{labels}}}");
//         match loc {
//             Some(s) => {
//                 let file = cache.source_name(cache.resolve_span(s).1);
//                 let (line_start, column_start) = cache.line_column(s.lo);
//                 let (line_end, column_end) = cache.line_column(s.hi);
//                 let _ = write!(out, "{file}:{line_start}:{column_start}..{line_end}:{column_end} {diag_str}");
//             }
//             None => {
//                 let _ = write!(out, "{diag_str}");
//             }
//         }
//         out
//     }
// }
