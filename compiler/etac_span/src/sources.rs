use std::{
    fmt, io,
    ops::Range,
    sync::{LazyLock, Mutex},
};

use ariadne::{Cache, Source};
use elsa::sync::FrozenMap;

use crate::{FileId, ReportableSpan, Span};

/// The process-wide [`SourceCache`]
///
/// One per process, never evicted.
static SOURCES: LazyLock<SourceCache> = LazyLock::new(SourceCache::new);

/// The global [`SOURCES`] cache, as the `&'static` borrow everything plumbs.
#[must_use]
pub fn sources() -> &'static SourceCache {
    &SOURCES
}

/// A loaded file, ariadne [`Source`] (which owns the text and the
/// precomputed line table) and its base offset in the global space.
struct CachedSource {
    source: Source<String>,
    base: u32,
}

impl CachedSource {
    fn text(&self) -> &str {
        self.source.text()
    }
}

#[derive(Default)]
struct AllocIndex {
    entries: Vec<(u32, FileId)>,
    next_base: u32,
}

/// Owns every loaded file, hands each a disjoint slice of the global byte space,
/// and resolves global [`Span`]s back to a file + local offsets. Doubles as
/// ariadne's [`Cache`].
///
/// Deliberately has no lifetime parameter: text borrows returned by
/// [`SourceCache::load`] / [`SourceCache::text`] are tied to the `&self` borrow
/// at the call site
pub struct SourceCache {
    /// Append-only. `FrozenMap::get`/`insert` take `&self` and return references
    /// that remain valid for the life of the cache; sync
    files: FrozenMap<FileId, Box<CachedSource>>,
    /// The base-offset allocator and the span index, guarded together so that
    /// assigning a base, recording it, and inserting the file stays one atomic
    /// step under concurrent `load`s. `entries` is `(base, id)`, ascending by
    /// base (bases are handed out in order), so `file_for` can binary-search.
    /// Borrows never escape the lock; values are copied out.
    alloc: Mutex<AllocIndex>,
}

impl SourceCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            files: FrozenMap::new(),
            alloc: Mutex::new(AllocIndex::default()),
        }
    }

    /// Read `id`
    ///
    /// If not already loaded: assign it a base.
    ///
    /// Returns the `(base, text)` corrosponding to the `id`.
    ///
    /// The returned `&str` borrows from the cache, so it may be held for as 
    /// long as the cache is alive.
    ///
    /// # Errors
    /// An IO error if we can not load the file from disk
    pub fn load(&self, id: FileId) -> io::Result<(u32, &str)> {
        let f = self.get_or_insert(id)?;
        Ok((f.base, f.text()))
    }

    /// Resolve a global span to its owning file and the local range within it.
    pub fn file_for(&self, span: Span) -> (FileId, Range<u32>) {
        let alloc = self.alloc.lock().expect("source cache poisoned");

        debug_assert!(!alloc.entries.is_empty(), "resolve() called before any file loaded");

        // the file with the greatest base <= span.lo contains the span
        let i = alloc
            .entries
            .partition_point(|(base, _)| *base <= span.lo)
            .saturating_sub(1);
        let (base, id) = alloc.entries[i];
        (id, (span.lo - base)..(span.hi - base))
    }

    /// Returns a span tied to this source-cache (this is useful for aridane 
    /// reporting since the library won't let you pass a SourceCache along with 
    /// your span)
    pub fn reportable_span_for(&self, span: Span) -> ReportableSpan<'_> {
        (self, span).into()
    }

    /// Full text of `id`. Loads the file if not already loaded.
    ///
    /// # Errors
    /// An IO error if we can not load the file see [`SourceCache::load`]
    pub fn text(&self, id: FileId) -> io::Result<&str> {
        Ok(self.load(id)?.1)
    }

    /// 1-based `(line, col)` for a global byte offset.
    ///
    /// # Errors
    /// An IO error if we can not load the file see [`SourceCache::load`]
    ///
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

        lc_from_ariadne_source(source, local_range.start as usize)
    }

    /// Load `id` if needed and return the cached entry. 
    ///
    /// The returned reference is tied to `&self`.
    fn get_or_insert(&self, id: FileId) -> io::Result<&CachedSource> {
        if let Some(f) = self.files.get(&id) {
            return Ok(f);
        }

        // Read before taking the allocator lock, so slow disk I/O never blocks
        // other threads' `file_for`/`lc_index` lookups.
        let raw = std::fs::read_to_string(id.as_str())?;

        // critical section below :)
        let mut alloc = self.alloc.lock().expect("source cache poisoned");
        if let Some(f) = self.files.get(&id) {
            return Ok(f);
        }
        let base = alloc.next_base;
        let len: u32 = raw.len().try_into().expect("source file exceeds 4 GiB");
        // +1 keeps adjacent files from sharing a boundary offset;
        let next = base
            .checked_add(len)
            .and_then(|end| end.checked_add(1))
            .expect("total loaded source exceeds 4 GiB");
        alloc.next_base = next;
        alloc.entries.push((base, id));
        Ok(self.files.insert(
            id,
            Box::new(CachedSource {
                source: Source::from(raw),
                base,
            }),
        ))
    }
}

impl Cache<FileId> for &SourceCache {
    type Storage = String;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<String>, impl fmt::Debug> {
        self.get_or_insert(*id).map(|cs| &cs.source)
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

#[doc(hidden)]
pub fn lc_from_ariadne_source(source: &ariadne::Source, at: usize) -> io::Result<(u32, u32)> {
    let (_line, linen, coln) = source
        .get_byte_line(at)
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
