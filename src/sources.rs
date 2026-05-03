use ariadne::{Cache, Source};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

pub mod span;
mod line_index;

use crate::error;
use crate::errors::Diagnostic;
use line_index::LineIndex;

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
    pub fn text(&mut self, id: &FileId) -> Result<Rc<str>, Diagnostic> {
        if let Some(rc) = self.texts.get(id) {
            return Ok(Rc::clone(rc));
        }
        let rc = std::fs::read_to_string(id.as_str())
            .map(Rc::from)
            .map_err(|e| error!(id, 0..0, "failed to read {id}: {e}"));
        if let Ok(rc1) = rc {
            self.texts.insert(id.clone(), Rc::clone(&rc1));
            Ok(rc1)
        } else { rc }
    }

    pub fn lc_index(&mut self, id: &FileId, offset: usize) -> Result<(usize, usize), Diagnostic> {
        if let Some(idx) = self.indexes.get(id) {
            return Ok(idx.line_col(offset));
        }
        let rc = std::fs::read_to_string(id.as_str())
            .map(Rc::from)
            .map_err(|e| error!(id, 0..0, "failed to read {id}: {e}"));
        if let Ok(rc1) = rc {
            self.texts.insert(id.clone(), Rc::clone(&rc1));
            let index = LineIndex::new(&rc1);
            let res = index.line_col(offset);
            self.indexes.insert(id.clone(), index);
            Ok(res)
        } else {
           Err(rc.expect_err("unreachable"))
        }
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
        Ok::<_, Diagnostic>(self.ariadne_sources.get(id).unwrap())
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        Some(id.as_str())
    }
}
