//! Source positions and the source map.
//!
//! All loaded files share one global byte-offset space: each file is assigned a
//! `base` and occupies `[base, base + len)`, with a one-byte gap between files.
//! A [`Span`] is therefore just two offsets into that space — 8 bytes, `Copy`,
//! and file-agnostic. The owning file is recovered on demand via
//! [`SourceCache::file_for`], so individual AST nodes never carry a [`FileId`].
//!
//! The space is addressed with `u32`, capping total loaded source at 4 GiB.
//!
//! The cache is append-only: files are inserted into an [`elsa::sync::FrozenMap`]
//! and never mutated or removed, so [`SourceCache::load`] can hand out `&str`
//! borrows tied to `&self` while later loads keep appending. Crucially,
//! [`SourceCache`] carries **no lifetime parameter** — borrowers (the lexer,
//! [`DiagCtxt`], ASTs) all borrow *from* the cache with their own ordinary
//! lifetimes, rather than the cache borrowing from itself.
//!
//! Both the path table behind [`FileId`] and the [`SourceCache`] are `Sync`:
//! ids and spans minted on one thread mean the same thing on every other, so
//! parallel per-file frontends can share one cache when the driver grows them.

use ariadne::{Cache, Source};
use elsa::sync::FrozenMap;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, RwLock};
use std::fmt;
use std::io;
use std::ops::Range;

/// A `Copy` handle naming a source or interface file.
///
/// The handle is a small index into a process-wide table of paths, so it can be
/// stored in maps and passed by value without cloning or borrowing. Recover the
/// original path with [`FileId::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u32);

/// file containing source code
pub type SourceId = FileId;
/// file containing an interface
pub type InterfaceId = FileId;

/// Interned file paths, one entry per distinct path seen this run. Process-global
/// rather than thread-local so a [`FileId`] minted on one thread resolves to the
/// same path on every other — a prerequisite for parallel per-file frontends.
/// Never cleared, and entries are leaked to `'static` so a [`FileId`] can return
/// its path as a plain `&str`. A compilation names only a handful of files, so
/// the table stays small and lives as long as the process.
static FILE_NAMES: LazyLock<RwLock<FileNames>> =
    LazyLock::new(|| RwLock::new(FileNames::default()));

#[derive(Default)]
struct FileNames {
    by_index: Vec<&'static str>,
    by_name: HashMap<&'static str, u32>,
}

impl FileNames {
    fn intern(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.by_name.get(name) {
            return id;
        }
        let name: &'static str = String::from(name).leak();
        #[allow(clippy::cast_possible_truncation)]
        let id = self.by_index.len() as u32;
        self.by_index.push(name);
        self.by_name.insert(name, id);
        id
    }
}

impl FileId {
    pub fn new(name: impl AsRef<str>) -> Self {
        FileId(
            FILE_NAMES
                .write()
                .expect("file-name table poisoned")
                .intern(name.as_ref()),
        )
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        // Entries are leaked to `'static`, so the borrow may outlive the guard.
        FILE_NAMES.read().expect("file-name table poisoned").by_index[self.0 as usize]
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A half-open byte range `[lo, hi)` in the global source space owned by
/// [`SourceCache`]. `Copy`, 8 bytes, and meaningless without the [`SourceCache`]
/// that minted it — use [`SourceCache::file_for`] to recover a
/// `(FileId, local range)`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    /// Placeholder span for synthesized nodes. Should never reach a diagnostic.
    pub const DUMMY: Span = Span { lo: 0, hi: 0 };

    pub fn new(lo: impl Into<u32>, hi: impl Into<u32>) -> Self {
        Self {
            lo: lo.into(),
            hi: hi.into(),
        }
    }

    /// Smallest span covering both `self` and `other`.
    #[must_use]
    pub fn to(self, other: Span) -> Span {
        Span {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }

    #[must_use]
    pub fn len(self) -> u32 {
        self.hi - self.lo
    }
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.lo == self.hi
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.lo, self.hi)
    }
}

// Enforces the size promised above; widening either field would regress every AST node.
const _: () = assert!(size_of::<Span>() == 8);

/// A loaded file: its ariadne [`Source`] (which owns the text and the
/// precomputed line table) and its base offset in the global space.
///
/// Boxed inside the [`FrozenMap`], so its address — and therefore any `&str`
/// handed out from it — stays stable as more files are loaded.
struct CachedSource {
    source: Source<String>,
    base: u32,
}

impl CachedSource {
    fn text(&self) -> &str {
        self.source.text()
    }
}

/// Owns every loaded file, hands each a disjoint slice of the global byte space,
/// and resolves global [`Span`]s back to a file + local range. Doubles as
/// ariadne's [`Cache`].
///
/// Deliberately has no lifetime parameter: text borrows returned by
/// [`SourceCache::load`] / [`SourceCache::text`] are tied to the `&self` borrow
/// at the call site, which is sound because the map is append-only and every
/// entry is boxed.
pub struct SourceCache {
    /// Append-only. `FrozenMap::get`/`insert` take `&self` and return references
    /// that remain valid for the life of the cache; the `sync` variant keeps that
    /// contract across threads.
    files: FrozenMap<FileId, Box<CachedSource>>,
    /// The base-offset allocator and the span index, guarded together so that
    /// assigning a base, recording it, and inserting the file stays one atomic
    /// step under concurrent `load`s. `entries` is `(base, id)`, ascending by
    /// base (bases are handed out in order), so `file_for` can binary-search.
    /// Borrows never escape the lock; values are copied out.
    alloc: Mutex<AllocIndex>,
}

#[derive(Default)]
struct AllocIndex {
    entries: Vec<(u32, FileId)>,
    next_base: u32,
}

impl SourceCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            files: FrozenMap::new(),
            alloc: Mutex::new(AllocIndex::default()),
        }
    }

    /// Read `id` (if not already loaded), assign it a base, and return
    /// `(base, text)`. The driver calls this before lexing so the lexer can
    /// shift its local positions into the global space.
    ///
    /// The returned `&str` borrows from the cache (not from a per-call value),
    /// so it may be held for as long as the cache is alive.
    /// # Errors
    /// An IO error if we can not load the file from disk
    pub fn load(&self, id: FileId) -> io::Result<(u32, &str)> {
        let f = self.ensure_loaded(id)?;
        Ok((f.base, f.text()))
    }

    /// Resolve a global span to its owning file and the local range within it.
    pub fn file_for(&self, span: Span) -> (FileId, Range<u32>) {
        let alloc = self.alloc.lock().expect("source cache poisoned");
        debug_assert!(
            !alloc.entries.is_empty(),
            "resolve() called before any file loaded"
        );
        // the file with the greatest base <= span.lo contains the span
        let i = alloc
            .entries
            .partition_point(|(base, _)| *base <= span.lo)
            .saturating_sub(1);
        let (base, id) = alloc.entries[i];
        (id, (span.lo - base)..(span.hi - base))
    }

    pub fn reportable_span_for(&self, span: Span) -> ReportableSpan<'_> {
        (self, span).into()
    }

    /// Full text of `id`; a map lookup on a cache hit.
    /// # Errors
    /// An IO error if we can not load the file see [`SourceCache::load`]
    pub fn text(&self, id: FileId) -> io::Result<&str> {
        Ok(self.load(id)?.1)
    }

    /// 1-based `(line, col)` for a global byte offset.
    /// # Errors
    /// An IO error if we can not load the file see [`SourceCache::load`]
    /// # Panics
    /// If offset is out of bounds of the virtual mega-file this function panics
    pub fn lc_index(&self, global_offset: u32) -> io::Result<(u32, u32)> {
        let (fileid, local_range) = self.file_for(Span {
            lo: global_offset,
            hi: global_offset,
        });
        let source = &self
            .files
            .get(&fileid)
            .expect("span resolved to a file that was never loaded")
            .source;
        let (_line, linen, coln) = source
            .get_byte_line(local_range.start as usize)
            .map(|(a, b, c)| {
                (
                    a,
                    u32::try_from(b).expect("requested line/col is out of bounds"),
                    u32::try_from(c).expect("requested line/col is out of bounds"),
                )
            })
            .expect("requested line/col is out of bounds");
        Ok((linen + 1, coln + 1))
    }

    /// Load `id` if needed and return the cached entry. The returned reference
    /// is tied to `&self`, i.e. it lives as long as the cache does.
    fn ensure_loaded(&self, id: FileId) -> io::Result<&CachedSource> {
        // Fast path: lock-free lookup for already-loaded files.
        if let Some(f) = self.files.get(&id) {
            return Ok(f);
        }
        // Read before taking the allocator lock, so slow disk I/O never blocks
        // other threads' `file_for`/`lc_index` lookups.
        let raw = std::fs::read_to_string(id.as_str())?;
        let mut alloc = self.alloc.lock().expect("source cache poisoned");
        // Two threads may race to load the same file. Every insert happens under
        // this lock, so the loser reliably finds the winner's entry here and
        // drops its own read; bases stay contiguous and are never leaked.
        if let Some(f) = self.files.get(&id) {
            return Ok(f);
        }
        let base = alloc.next_base;
        let len: u32 = raw.len().try_into().expect("source file exceeds 4 GiB");
        // +1 keeps adjacent files from sharing a boundary offset; the checked adds
        // also enforce the 4 GiB cap on the whole global space.
        let next = base
            .checked_add(len)
            .and_then(|end| end.checked_add(1))
            .expect("total loaded source exceeds 4 GiB");
        alloc.next_base = next;
        alloc.entries.push((base, id));
        // `FrozenMap::insert` takes `&self` and returns a reference to the
        // (boxed, address-stable) inserted value.
        Ok(self.files.insert(
            id,
            Box::new(CachedSource {
                source: Source::from(raw),
                base,
            }),
        ))
    }
}

impl SourceCache {
    /// Borrow this cache as an ariadne [`Cache`] without needing `&mut`.
    ///
    /// Unlike the previous `RefMut`-based view, this is just a shared borrow:
    /// any number of views may be live at once, and fetching a not-yet-loaded
    /// file loads it on demand. This is what lets a single shared
    /// `&SourceCache` back both the lexer and the diagnostic emitter.
    pub fn cache_view(&self) -> CacheView<'_> {
        CacheView { cache: self }
    }
}

/// A borrowed ariadne [`Cache`] view over a [`SourceCache`].
/// Created by [`SourceCache::cache_view`].
pub struct CacheView<'a> {
    cache: &'a SourceCache,
}

impl Cache<FileId> for CacheView<'_> {
    type Storage = String;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<String>, impl fmt::Debug> {
        // loads on demand; the returned `&Source` is tied to `&mut self`, so it
        // lives exactly as long as ariadne needs it within this call
        self.cache.ensure_loaded(*id).map(|cs| &cs.source)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        Some(id.as_str())
    }
}

impl Cache<FileId> for SourceCache {
    type Storage = String;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<String>, impl fmt::Debug> {
        self.ensure_loaded(*id).map(|cs| &cs.source)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        Some(id.as_str())
    }
}

impl Default for SourceCache {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ReportableSpan<'a> {
    cache: &'a SourceCache,
    pub span: Span,
    own: std::cell::OnceCell<(FileId, Range<u32>)>,
}

impl<'a> From<(&'a SourceCache, Span)> for ReportableSpan<'a> {
    fn from(value: (&'a SourceCache, Span)) -> Self {
        ReportableSpan {
            cache: value.0,
            span: value.1,
            own: std::cell::OnceCell::new(),
        }
    }
}

impl ariadne::Span for ReportableSpan<'_> {
    type SourceId = FileId;

    fn source(&self) -> &Self::SourceId {
        &self.own.get_or_init(|| self.cache.file_for(self.span)).0
    }

    fn start(&self) -> usize {
        self.own.get_or_init(|| self.cache.file_for(self.span)).1.start as usize
    }

    fn end(&self) -> usize {
        self.own.get_or_init(|| self.cache.file_for(self.span)).1.end as usize
    }
}

// Parallel per-file frontends require sharing the cache (and passing FileIds)
// across threads; keep that possibility honest at compile time.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SourceCache>();
    assert_send_sync::<FileId>();
    assert_send_sync::<Span>();
};
