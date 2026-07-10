use std::{ops::{Bound, Range}, sync::{LazyLock, atomic::{AtomicU32, Ordering}}};
use elsa::sync::FrozenMap;
use dashmap::DashMap;
use crossbeam_skiplist::SkipMap;
use crate::{FileId, Span, sources::SourceCache};

/// The process-wide [`SourceCache`]
///
/// One per process, never evicted.
static SOURCES: LazyLock<GlobalCache> = LazyLock::new(GlobalCache::default);

/// The global [`SOURCES`] cache, as the `&'static` borrow everything plumbs.
#[must_use]
pub fn sources() -> &'static GlobalCache {
    &SOURCES
}

#[derive(Default)]
pub struct GlobalCache {
    files: FrozenMap<FileId, Box<ariadne::Source<String>>>,
    by_name: DashMap<&'static str, FileId>,
    by_offset: SkipMap<u32, &'static str>,
    alloc: AtomicU32,
}

impl SourceCache for GlobalCache {
    fn contains(&self, display_name: &str) -> Option<FileId> {
        self.by_name.get(display_name).map(|e| *e.value())
    }

    fn store(&mut self, display_name: String, value: String) -> (FileId, &ariadne::Source<String>) {
        let value_bytes = value.len() as u32;
        let fileid = FileId(self.alloc.fetch_add(value_bytes, Ordering::SeqCst));
        let name: &'static str = display_name.leak();
        self.by_name.insert(name, fileid);
        self.by_offset.insert(fileid.0, name);
        let source = ariadne::Source::from(value);
        let source_ref = self.files.insert(fileid, Box::new(source));
        (fileid, source_ref)
    }

    fn load_source(&self, id: FileId) -> &ariadne::Source<String> {
        self.files.get(&id).expect("FileId constructed outside this cache passed")
    }

    fn load_name(&self, id: FileId) -> &str {
        self.by_offset.get(&id.0).expect("FileId constructed outside this cache passed").value()
    }

    fn resolve_span(&self, span: Span) -> (Range<u32>, FileId) {
        let entry = self
            .by_offset
            .upper_bound(Bound::Included(&span.lo))
            .expect("span.lo below the first file start");
        let base = entry.key();
        ((span.lo - base)..(span.hi - base), FileId(*base))
    }
}
