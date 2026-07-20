//! Diagnostic sinks.
//!
//! An [`Emitter`] is the *only* thing that turns a [`Diagnostic`] into output. The
//! [`DiagCtxt`](crate::DiagCtxt) owns one and routes every diagnostic through it

use std::{cell::RefCell, collections::HashMap, io::Write, rc::Rc};

use ariadne::{Config, IndexType, Label, Report, ReportKind};
use etac_cache::sources::{FileId, SourceMap};

use crate::{Level};

/// Can take ownership of a diagnostic to emit it
pub trait Emitter {
    fn emit<'sm>(&mut self, cache: &mut EmitCache<'sm>, diag: crate::dcx::Diagnostic);
}
pub struct EmitCache<'sm> {
    sm: &'sm SourceMap,
    cache: HashMap<FileId<'sm>, ariadne::Source<&'sm str>>,
}
impl<'sm> EmitCache<'sm> {
    pub fn new(sm: &'sm SourceMap) -> Self {
        Self {
            sm,
            cache: HashMap::new(),
        }
    }
}

impl<'sm> ariadne::Cache<FileId<'sm>> for &mut EmitCache<'sm> {
    type Storage = &'sm str;

    fn fetch(&mut self, id: &FileId<'sm>) -> Result<&ariadne::Source<Self::Storage>, impl std::fmt::Debug> {
        if !self.cache.contains_key(id) {
            self.cache.insert(*id, ariadne::Source::from(self.sm.get_source(*id).source.as_str()));
        }
        Ok::<_, std::convert::Infallible>(self.cache.get(id).unwrap())
    }

    fn display<'a>(&self, id: &'a FileId<'sm>) -> Option<impl std::fmt::Display + 'a> {
        Some(self.sm.get_source(*id).name.clone())
    }
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
    fn emit<'sm>(&mut self, cache: &mut EmitCache<'sm>, diag: crate::dcx::Diagnostic) {
        let kind = match diag.level {
            Level::Error => ReportKind::Error,
            Level::Warning => ReportKind::Warning,
            Level::Note => ReportKind::Advice,
        };

        if let Some(loc) = diag.loc {
            let byte_config = Config::default().with_index_type(IndexType::Byte);
            let aspan = cache.sm.local_span(loc);
            let mut b = Report::build(kind, aspan)
                .with_config(byte_config)
                .with_message(&diag.message);
            if let Some(c) = &diag.code {
                b = b.with_code(c);
            }
            if let Some(n) = &diag.note {
                b = b.with_note(n);
            }
            for (span, msg, color) in &diag.labels {
                let aspan = cache.sm.local_span(*span);
                b = b.with_label(Label::new(aspan).with_message(msg).with_color(*color));
            }
            let _ = b.finish().write(cache, &mut self.writer);
        } else {
            let mut b = Report::build(kind, ("dummy", 0..0)).with_message(&diag.message);
            if let Some(c) = &diag.code {
                b = b.with_code(c);
            }
            if let Some(n) = &diag.note {
                b = b.with_note(n);
            }
            for (_span, msg, color) in &diag.labels {
                b = b.with_label(Label::new(("dummy", 0..0)).with_message(msg).with_color(*color));
            }
            let _ = b.finish().write(ariadne::sources::<_, _, [(&str, &str); 0]>([]), &mut self.writer);
        }
    }
}

/// Records diagnostics into a shared buffer instead of printing them.
///
/// Cloning shares the same underlying buffer (it is an `Rc<RefCell<_>>`), so a test can
/// keep a handle, hand a clone to the [`DiagCtxt`](crate::DiagCtxt), run a phase, and
/// then read back exactly what was emitted via [`take`](BufferEmitter::take).
#[derive(Debug, Clone, Default)]
pub struct BufferEmitter(Rc<RefCell<Vec<crate::dcx::Diagnostic>>>);

impl BufferEmitter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain everything emitted so far, leaving the buffer empty.
    #[must_use]
    pub fn take(&self) -> Vec<crate::dcx::Diagnostic> {
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

impl Emitter for BufferEmitter {
    fn emit(&mut self, _source_map: &mut EmitCache, diagnostic: crate::dcx::Diagnostic) {
        self.0.borrow_mut().push(diagnostic);
    }
}
