use ariadne::{Cache, Source};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
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

pub struct CachedSource {
    rc: Rc<str>,
    source: Source<Rc<str>>,
}

pub struct Sources {
    ariadne_sources: RefCell<HashMap<FileId, CachedSource>>,
}

impl Cache<FileId> for Sources {
    type Storage = Rc<str>;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<Rc<str>>, impl fmt::Debug> {
        let map = self.ariadne_sources.get_mut();
        if !map.contains_key(id) {
            let rc: Rc<str> = std::fs::read_to_string(id.as_str()).map(Rc::from)?;
            map.insert(id.clone(), CachedSource { rc: Rc::clone(&rc), source: Source::from(rc) });
        }
        Ok::<_, std::io::Error>(&map.get(id).unwrap().source)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        Some(id.as_str())
    }
}

impl Sources {
    pub fn new() -> Self {
        Self { ariadne_sources: RefCell::new(HashMap::new()) }
    }

    /// Returns Rc<str> — just a pointer bump on cache hit, and
    /// callers can deref as &str whenever they need it.
    pub fn text(&self, id: &FileId) -> Result<Rc<str>, std::io::Error> {
        self.ensure_exists(id)?;
        // Borrow lives only for this statement; we clone the Rc before it drops.
        Ok(Rc::clone(&self.ariadne_sources.borrow().get(id).unwrap().rc))
    }

    pub fn lc_index(&self, id: &FileId, offset: usize) -> Result<(usize, usize), std::io::Error> {
        self.ensure_exists(id)?;
        let map = self.ariadne_sources.borrow();
        let source = &map.get(id).unwrap().source;
        // zero indexed
        let (_line, linen, coln) = source
            .get_offset_line(offset)
            .expect("requested line/col is out of bounds");
        Ok((linen+1, coln+1))
    }

    fn ensure_exists(&self, id: &FileId) -> Result<(), std::io::Error> {
        if !self.ariadne_sources.borrow().contains_key(id) {
            let rc: Rc<str> = std::fs::read_to_string(id.as_str()).map(Rc::from)?;
            self.ariadne_sources.borrow_mut()
                .insert(id.clone(), CachedSource { rc: Rc::clone(&rc), source: Source::from(rc) });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtaSpan {
    pub file_id: SourceId,
    pub range: std::ops::Range<usize>,
}

impl EtaSpan {
    pub fn new(file_id: SourceId, l: usize, r: usize) -> Self {
        Self { file_id, range: (l..r) }
    }
}

impl From<(&SourceId, std::ops::Range<usize>)> for EtaSpan {
    fn from((file_id, range): (&SourceId, std::ops::Range<usize>)) -> Self {
        EtaSpan { file_id: file_id.clone(), range }
    }
}

/// So that ariadne can report errors given EtaSpans
impl ariadne::Span for EtaSpan {
    type SourceId = SourceId;
    fn source(&self) -> &SourceId { &self.file_id }
    fn start(&self) -> usize   { self.range.start }
    fn end(&self)   -> usize   { self.range.end }
}

// for aridane 

impl Default for Sources {
    fn default() -> Self { Self::new() }
}


// for Logos
impl Default for FileId {
    fn default() -> Self {
        Self(Default::default())
    }
}
