//! Source positions and the source map.
//!
//! All loaded files share one global byte-offset space: each file is assigned a
//! `base` and occupies `[base, base + len)`, with a one-byte gap between files.
//! A [`Span`] is therefore just two offsets into that space — 8 bytes, `Copy`,
//! and file-agnostic. The owning file is recovered on demand via
//! [`Sources::resolve`], so individual AST nodes never carry a `FileId`.

use ariadne::{Cache, Source};
use std::borrow::Borrow;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::ops::Range;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// use when you don't care about the difference between interface and source files
pub struct FileId(Rc<str>);

/// file containing source code
pub type SourceId = FileId;
/// file containing an interface
pub type InterfaceId = FileId;

impl FileId {
    pub fn new(name: impl Into<Rc<str>>) -> Self { Self(name.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

// Lets HashMap<FileId, _> be queried with &str directly:
impl Borrow<str> for FileId {
    fn borrow(&self) -> &str { &self.0 }
}

/// A half-open byte range `[lo, hi)` in the global source space owned by
/// [`Sources`]. `Copy`, 8 bytes, and meaningless without the `Sources` that
/// minted it — use [`Sources::resolve`] to recover a `(FileId, local range)`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
}

impl Span {
    /// Placeholder span for synthesized nodes. Should never reach a diagnostic.
    pub const DUMMY: Span = Span { lo: 0, hi: 0 };

    pub fn new(lo: impl Into<usize>, hi: impl Into<usize>) -> Self { Self { lo: lo.into(), hi: hi.into() } }

    /// Smallest span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span { lo: self.lo.min(other.lo), hi: self.hi.max(other.hi) }
    }

    pub fn len(self) -> usize { self.hi - self.lo }
    pub fn is_empty(self) -> bool { self.lo == self.hi }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.lo, self.hi)
    }
}

/// Temporary alias so existing `EtaSpan` *type* references keep compiling during
/// the Phase 1 migration. Delete once the Phase 2 AST refactor lands.
pub type EtaSpan = Span;

struct CachedSource {
    rc: Rc<str>,
    source: Source<Rc<str>>,
    base: usize,
}

/// Owns every loaded file, hands each a disjoint slice of the global byte space,
/// and resolves global [`Span`]s back to a file + local range. Doubles as
/// ariadne's [`Cache`].
pub struct SourceCache {
    files: RefCell<HashMap<FileId, CachedSource>>,
    /// `(base, id)` kept sorted ascending by base (bases are handed out in
    /// order), so `resolve` can binary-search.
    index: RefCell<Vec<(usize, FileId)>>,
    next_base: Cell<usize>,
}

impl SourceCache {
    pub fn new() -> Self {
        Self {
            files: RefCell::new(HashMap::new()),
            index: RefCell::new(Vec::new()),
            next_base: Cell::new(0),
        }
    }

    /// Read `id` (if not already loaded), assign it a base, and return
    /// `(base, text)`. The driver calls this before lexing so the lexer can
    /// shift its local positions into the global space.
    pub fn load(&self, id: &FileId) -> io::Result<(usize, Rc<str>)> {
        self.ensure_loaded(id)?;
        let files = self.files.borrow();
        let f = files.get(id).expect("just loaded");
        Ok((f.base, Rc::clone(&f.rc)))
    }

    /// Resolve a global span to its owning file and the local range within it.
    pub fn resolve(&self, span: Span) -> (FileId, Range<usize>) {
        let index = self.index.borrow();
        debug_assert!(!index.is_empty(), "resolve() called before any file loaded");
        // the file with the greatest base <= span.lo contains the span
        let i = index.partition_point(|(base, _)| *base <= span.lo).saturating_sub(1);
        let (base, id) = &index[i];
        ((*id).clone(), (span.lo - base) as usize..(span.hi - base) as usize)
    }

    /// Full text of `id`; a pointer bump on a cache hit.
    pub fn text(&self, id: &FileId) -> io::Result<Rc<str>> {
        Ok(self.load(id)?.1)
    }

    /// 1-based `(line, col)` for a *local* byte offset within `id`.
    pub fn file_lc_index(&self, id: &FileId, offset: usize) -> io::Result<(usize, usize)> {
        self.ensure_loaded(id)?;
        let map = self.files.borrow();
        let source = &map.get(id).expect("just loaded").source;
        let (_line, linen, coln) = source
            .get_byte_line(offset)
            .expect("requested line/col is out of bounds");
        Ok((linen + 1, coln + 1))
    }

    pub fn lc_index(&self, global_offset: usize) -> io::Result<(usize, usize)> {
        let (fileid, local_range) = self.resolve(Span::new(global_offset, global_offset));
        let map = self.files.borrow();
        let source = &map.get(&fileid).unwrap().source;
        let (_line, linen, coln) = source
            .get_byte_line(local_range.start)
            .expect("requested line/col is out of bounds");
        Ok((linen + 1, coln + 1))
    }

    fn ensure_loaded(&self, id: &FileId) -> io::Result<()> {
        if self.files.borrow().contains_key(id) {
            return Ok(());
        }
        let rc: Rc<str> = std::fs::read_to_string(id.as_str()).map(Rc::from)?;
        let base = self.next_base.get();
        let len: u32 = rc.len().try_into().expect("source file exceeds 4 GiB");
        // +1 keeps adjacent files from sharing a boundary offset.
        self.next_base.set(base + (len as usize) + 1);
        self.index.borrow_mut().push((base, id.clone()));
        self.files.borrow_mut().insert(
            id.clone(),
            CachedSource { rc: Rc::clone(&rc), source: Source::from(rc), base },
        );
        Ok(())
    }
}

impl SourceCache {
    /// Borrow this cache as an ariadne [`Cache`] without needing `&mut`.
    ///
    /// The returned view holds an interior [`RefMut`] for its whole lifetime, so at
    /// most one may be live at a time (one diagnostic renders at a time). Every file
    /// referenced by the report must already be loaded — which it always is by the
    /// time we render a diagnostic, since the span pointing at it could only have been
    /// minted by lexing that file. This is what lets a single shared `&SourceCache`
    /// back both the lexer and the diagnostic emitter.
    pub fn cache_view(&self) -> CacheView<'_> {
        CacheView { files: self.files.borrow_mut() }
    }
}

/// A borrowed, interior-mutable ariadne [`Cache`] view over a [`SourceCache`].
/// Created by [`SourceCache::cache_view`].
pub struct CacheView<'a> {
    files: RefMut<'a, HashMap<FileId, CachedSource>>,
}

impl Cache<FileId> for CacheView<'_> {
    type Storage = Rc<str>;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<Rc<str>>, impl fmt::Debug> {
        // `self.files.get` borrows from `self` (tied to `&mut self`), so the returned
        // `&Source` lives exactly as long as ariadne needs it within this call.
        self.files
            .get(id)
            .map(|cs| &cs.source)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("source not loaded: {id}")))
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        Some(id.as_str())
    }
}

impl Cache<FileId> for SourceCache {
    type Storage = Rc<str>;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<Rc<str>>, impl fmt::Debug> {
        self.ensure_loaded(id)?;
        Ok::<_, io::Error>(&self.files.get_mut().get(id).expect("just loaded").source)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        Some(id.as_str())
    }
}

impl Default for SourceCache {
    fn default() -> Self { Self::new() }
}

// for Logos
impl Default for FileId {
    fn default() -> Self {
        Self(Default::default())
    }
}
