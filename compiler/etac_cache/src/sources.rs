use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use crossbeam_skiplist::SkipMap;
use dashmap::DashMap;
use elsa::sync::FrozenMap;

pub type SourceId<'sm> = FileId<'sm>;
pub type InterfaceId<'sm> = FileId<'sm>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FileId<'sm> {
    pub(crate) base: u32,
    _cache: PhantomData<&'sm SourceMap>,
}
impl FileId<'_> {
    pub(crate) fn new(base: u32) -> Self {
        FileId {
            base,
            _cache: PhantomData,
        }
    }
}

pub struct SourceRecord {
    pub name: Rc<str>,
    pub source: String,
}

#[derive(Default)]
pub struct SourceMap {
    source_starts: SkipMap<u32, ()>,
    source_texts: FrozenMap<u32, Box<SourceRecord>>,
    name_to_id: DashMap<String, u32>,
    next_base: AtomicU32,
}

impl SourceMap {
    pub fn alloc_source(&self, display_name: String, text: String) -> (FileId<'_>, &str) {
        let len = u32::try_from(text.len()).expect("text maximum is 4GB (u32::MAX bytes)");
        // +1 keeps bases unique even for empty files.
        let base = self.next_base.fetch_add(len + 1, Ordering::SeqCst);
        self.name_to_id.insert(display_name.clone(), base);
        self.source_starts.insert(base, ());
        let record = self.source_texts.insert(
            base,
            Box::new(SourceRecord {
                name: display_name.into(),
                source: text,
            }),
        );
        (FileId::new(base), &record.source)
    }

    pub fn get_id(&self, display_name: &str) -> Option<FileId<'_>> {
        self.name_to_id.get(display_name).map(|e| FileId::new(*e.value()))
    }

    pub fn get_source<'ec>(&'ec self, id: FileId<'ec>) -> &'ec SourceRecord {
        self.source_texts
            .get(&id.base)
            .expect("FileId constructed outside this cache passed")
    }

    pub fn base_offset(&self, id: FileId<'_>) -> u32 {
        id.base
    }

    pub fn local_span(&self, span: Span) -> (FileId<'_>, std::ops::Range<usize>) {
        let entry = self
            .source_starts
            .upper_bound(std::ops::Bound::Included(&span.lo))
            .expect("span.lo below the first file start");
        let base = *entry.key();
        let id = FileId::new(base);
        (id, (span.lo - base) as usize..(span.hi - base) as usize)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}
impl Span {
    pub fn new(lo: impl Into<u32>, hi: impl Into<u32>) -> Self {
        Self { lo: lo.into(), hi: hi.into(), }
    }
}

