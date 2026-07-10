use std::{fmt, ops::{Bound, Range}, sync::{LazyLock, atomic::{AtomicU32, Ordering}}};
use elsa::sync::FrozenMap;
use dashmap::DashMap;
use crossbeam_skiplist::SkipMap;
use ariadne::{Source};
use crate::{FileId, ReportableSpan, Span};

static SOURCES: LazyLock<SCache> = LazyLock::new(SCache::default);

/// The global [`SOURCES`] cache, as the `&'static` borrow.
pub fn sources() -> &'static SCache {
    &SOURCES
}

#[derive(Default)]
pub struct SCache {
    files: FrozenMap<FileId, Box<ariadne::Source<String>>>,
    by_name: DashMap<&'static str, FileId>,
    by_offset: SkipMap<u32, &'static str>,
    alloc: AtomicU32,
}

impl SCache {
    fn contains(&self, display_name: &str) -> Option<FileId> {
        self.by_name.get(display_name).map(|e| *e.value())
    }

    fn store(&self, display_name: String, value: String) -> (FileId, &ariadne::Source<String>) {
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

    fn load(&self, id: FileId) -> (u32, &ariadne::Source<String>) {
        (id.0, self.load_source(id))
    }

    fn reportable_span(&self, span: Span) -> ReportableSpan<'_, Self> {
        ReportableSpan::new(self, span)
    }

    fn line_column(&self, global_offset: u32) -> (u32, u32) {
        let (local_range, file_id) = self.resolve_span(Span::new(global_offset, global_offset));
        let source = self.load_source(file_id);
        let (_line, linen, coln) = source
            .get_byte_line(local_range.start as usize)
            .expect("requested line/col is out of bounds");
        (
            u32::try_from(linen).expect("requested line/col is out of bounds") + 1,
            u32::try_from(coln).expect("requested line/col is out of bounds") + 1,
        )
    }
}

impl<'a> ariadne::Cache<FileId> for &'a SCache {
    type Storage = String;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<Self::Storage>, impl fmt::Debug> {
        Ok::<_, std::convert::Infallible>(self.0.load_source(*id))
    }

    fn display<'b>(&self, id: &'b FileId) -> Option<impl fmt::Display + 'b> {
        Some(self.0.load_name(*id).to_owned())
    }
}
