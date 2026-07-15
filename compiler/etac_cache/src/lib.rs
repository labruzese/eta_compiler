//! The central store for everything long-lived in a compilation.
//!
//! An [`EtaCache`] owns the source text, file names, per-node spans, and
//! parsed trees for one compilation. Phases hold `&'ec EtaCache` and query it;
//! everything it hands back — [`FileId`]s included — is tied to that borrow,
//! so nothing can outlive the compilation that produced it.
//!
//! All storage is append-only behind `&self`: a returned reference stays valid
//! for the rest of the compilation no matter what is stored afterwards.

use std::{
    fmt,
    marker::PhantomData,
    ops::{Bound, Range},
    sync::atomic::{AtomicU32, Ordering},
};

use crossbeam_skiplist::SkipMap;
use dashmap::DashMap;
use elsa::sync::FrozenMap;
use etac_ast::{AstNode, Interface, NodeId, Program};

mod node_spans;
mod reportable;
mod span;

use node_spans::SpanTable;
pub use reportable::ReportableSpan;
pub use span::Span;

/// Key for a file stored in an [`EtaCache`].
///
/// The lifetime ties the id to the cache borrow: a `FileId`
/// cannot outlive its cache, and only that cache can resolve it.
///
/// Internally it is the file's base offset in the cache's global span space.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId<'ec> {
    base: u32,
    _cache: PhantomData<&'ec EtaCache>,
}

pub type SourceId<'ec> = FileId<'ec>;
pub type InterfaceId<'ec> = FileId<'ec>;

impl FileId<'_> {
    fn new(base: u32) -> Self {
        FileId {
            base,
            _cache: PhantomData,
        }
    }
}

impl fmt::Debug for FileId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileId({})", self.base)
    }
}

struct FileRecord {
    name: String,
    source: ariadne::Source<String>,
}

#[derive(Default)]
pub struct EtaCache {
    files: FrozenMap<u32, Box<FileRecord>>,
    by_name: DashMap<String, u32>,
    bases: SkipMap<u32, ()>,
    next_base: AtomicU32,
    node_spans: SpanTable,
    programs: FrozenMap<u32, Box<Program>>,
    interfaces: FrozenMap<u32, Box<Interface>>,
}

impl EtaCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // ---- sources ----

    /// Store a file's text under `display_name`, minting its [`FileId`].
    ///
    /// Storing the same name twice mints a second id over a second copy; use
    /// [`source_id`](Self::source_id) first to reuse an existing one.
    pub fn store_source(&self, display_name: String, text: String) -> (FileId<'_>, &ariadne::Source<String>) {
        let len = u32::try_from(text.len()).expect("text maximum is 4GB (u32::MAX bytes)");
        // +1 keeps bases unique even for empty files.
        let base = self.next_base.fetch_add(len + 1, Ordering::SeqCst);
        self.by_name.insert(display_name.clone(), base);
        self.bases.insert(base, ());
        let record = self.files.insert(
            base,
            Box::new(FileRecord {
                name: display_name,
                source: ariadne::Source::from(text),
            }),
        );
        (FileId::new(base), &record.source)
    }

    /// The id already minted for `display_name`, if any.
    pub fn source_id(&self, display_name: &str) -> Option<FileId<'_>> {
        self.by_name.get(display_name).map(|e| FileId::new(*e.value()))
    }

    /// # Panics
    /// If `id` was minted by a different cache.
    pub fn source<'ec>(&'ec self, id: FileId<'ec>) -> &'ec ariadne::Source<String> {
        &self.record(id).source 
    }

    /// # Panics
    /// If `id` was minted by a different cache.
    pub fn source_text<'ec>(&'ec self, id: FileId<'ec>) -> &'ec str {
        self.source(id).text()
    }

    /// # Panics
    /// If `id` was minted by a different cache.
    pub fn source_name<'ec>(&'ec self, id: FileId<'ec>) -> &'ec str {
        &self.record(id).name
    }

    /// The file's base offset in the global span space;
    pub fn base_offset(&self, id: FileId<'_>) -> u32 {
        id.base
    }

    fn record<'ec>(&'ec self, id: FileId<'ec>) -> &'ec FileRecord {
        self.files
            .get(&id.base)
            .expect("FileId constructed outside this cache passed")
    }

    // ---- span resolution ----

    /// The file containing `span` and the span's byte range local to it.
    ///
    /// # Panics
    /// If `span` starts before the first stored file.
    pub fn resolve_span(&self, span: Span) -> (Range<u32>, FileId<'_>) {
        let entry = self
            .bases
            .upper_bound(Bound::Included(&span.lo))
            .expect("span.lo below the first file start");
        let base = *entry.key();
        ((span.lo - base)..(span.hi - base), FileId::new(base))
    }

    pub fn reportable_span(&self, span: Span) -> ReportableSpan<'_> {
        ReportableSpan::new(self, span)
    }

    /// 1-based line and column of a global byte offset.
    ///
    /// # Panics
    /// If the offset falls outside every stored file.
    pub fn line_column(&self, global_offset: u32) -> (u32, u32) {
        let (local_range, file_id) = self.resolve_span(Span::new(global_offset, global_offset));
        let source = self.source(file_id);
        let (_line, linen, coln) = source
            .get_byte_line(local_range.start as usize)
            .expect("requested line/col is out of bounds");
        (
            u32::try_from(linen).expect("requested line/col is out of bounds") + 1,
            u32::try_from(coln).expect("requested line/col is out of bounds") + 1,
        )
    }

    // ---- node spans ----

    /// Mint a fresh [`NodeId`] with `span` recorded for it.
    pub fn alloc_span(&self, span: Span) -> NodeId {
        self.node_spans.alloc(span)
    }

    /// Fresh id sharing the span already recorded for `of`. For wrapper nodes
    /// and reinterpretations that cover the same source text.
    pub fn dup_span(&self, of: NodeId) -> NodeId {
        self.node_spans.alloc(self.node_spans.get(of))
    }

    /// The span recorded for `id`. [`NodeId::DUMMY`] maps to [`Span::DUMMY`].
    ///
    /// # Panics
    /// If `id` was allocated by a different cache.
    pub fn span(&self, id: NodeId) -> Span {
        self.node_spans.get(id)
    }

    pub fn span_of(&self, node: &impl AstNode) -> Span {
        self.span(node.node_id())
    }

    // ---- parsed trees ----

    pub fn store_program<'ec>(&'ec self, id: FileId<'ec>, program: Program) -> &'ec Program {
        self.programs.insert(id.base, Box::new(program))
    }

    pub fn program<'ec>(&'ec self, id: FileId<'ec>) -> Option<&'ec Program> {
        self.programs.get(&id.base)
    }

    pub fn store_interface<'ec>(&'ec self, id: FileId<'ec>, interface: Interface) -> &'ec Interface {
        self.interfaces.insert(id.base, Box::new(interface))
    }

    pub fn interface<'ec>(&'ec self, id: FileId<'ec>) -> Option<&'ec Interface> {
        self.interfaces.get(&id.base)
    }
}

impl fmt::Debug for EtaCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EtaCache")
            .field("files", &self.by_name.len())
            .finish_non_exhaustive()
    }
}

impl<'ec> ariadne::Cache<FileId<'ec>> for &'ec EtaCache {
    type Storage = String;

    fn fetch(&mut self, id: &FileId<'ec>) -> Result<&ariadne::Source<Self::Storage>, impl fmt::Debug> {
        Ok::<_, std::convert::Infallible>(self.source(*id))
    }

    fn display<'a>(&self, id: &'a FileId<'ec>) -> Option<impl fmt::Display + 'a> {
        Some(self.source_name(*id).to_owned())
    }
}
