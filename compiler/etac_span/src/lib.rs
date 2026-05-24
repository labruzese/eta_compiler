use ariadne::{Cache, Source};
use std::borrow::Borrow;
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

pub struct Sources {
    texts: HashMap<FileId, Rc<str>>,
    indexes: HashMap<FileId, LineIndex>,
    ariadne_sources: HashMap<FileId, Source<Rc<str>>>,
}

impl Sources {
    pub fn new() -> Self {
        Self {
            texts: HashMap::new(),
            indexes: HashMap::new(),
            ariadne_sources: HashMap::new(),
        }
    }

    /// Cheap: `Rc` bump on cache hit, disk read + insert on miss.
    /// Panics if the file can't be read
    pub fn text(&mut self, id: &FileId) -> Result<Rc<str>, std::io::Error> {
        if let Some(rc) = self.texts.get(id) {
            return Ok(Rc::clone(rc));
        }
        let rc = std::fs::read_to_string(id.as_str())
            .map(Rc::from)
            ?;

        self.texts.insert(id.clone(), Rc::clone(&rc));
        Ok(rc)
    }

    // provides the char line:col (not byte offset line:col)
    pub fn lc_index(&mut self, id: &FileId, offset: usize) -> Result<(usize, usize), std::io::Error> {
        if let Some(idx) = self.indexes.get(id) {
            let text = self.texts.get(id).expect("text always inserted with index");
            return Ok(idx.line_col(offset, text));
        }
        let rc = std::fs::read_to_string(id.as_str())
            .map(Rc::from)
            ?;

        self.texts.insert(id.clone(), Rc::clone(&rc));
        let index = LineIndex::new(&rc);
        let res = index.line_col(offset, &rc);
        self.indexes.insert(id.clone(), index);
        Ok(res)
    }

    /// Inject a source directly — handy for tests and for sources that
    /// don't correspond to a real file on disk.
    #[allow(unused)]
    pub fn insert(&mut self, id: FileId, text: impl Into<Rc<str>>) {
        let rc = text.into();
        self.texts.insert(id.clone(), rc);
        // Drop any stale ariadne Source so it gets rebuilt on next fetch.
        self.ariadne_sources.remove(&id);
    }

    #[allow(unused)]
    pub fn text_by_name(&self, name: &str) -> Option<Rc<str>> {
        self.texts.get(name).map(Rc::clone)
    }
}


/// Precomputed byte-offset -> (line, column) map for one source text.
/// Build: O(n) scan. Query: O(log n).
#[derive(Debug)]
pub struct LineIndex {
    /// Byte offset of the first character on each line (1-indexed in output).
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = Vec::with_capacity(text.len() / 40 + 1);
        line_starts.push(0);
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// 1-indexed (line, column). Column is a **character** offset,
    /// suitable for display to users.
    pub fn line_col(&self, offset: usize, text: &str) -> (usize, usize) {
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_idx];
        // count characters, not bytes
        let char_col = text[line_start..offset]
            .chars()
            .map(|c| unicode_width::UnicodeWidthChar::width_cjk(c).unwrap_or(0))
            .sum::<usize>();
        (line_idx + 1, char_col + 1)
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

impl Cache<FileId> for Sources {
    type Storage = Rc<str>;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<Rc<str>>, impl fmt::Debug> {
        if !self.ariadne_sources.contains_key(id) {
            let rc = self.text(id)?;
            self.ariadne_sources.insert(id.clone(), Source::from(rc));
        }
        Ok::<_, std::io::Error>(self.ariadne_sources.get(id).unwrap())
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        Some(id.as_str())
    }
}
