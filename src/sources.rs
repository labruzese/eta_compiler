use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::{self, Display};
use std::path::Path;
use std::rc::Rc;

/// Span is a file and byte range
mod span;
pub use span::*;

/// A unique identifier for a source file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(String);

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

/// Concrete struct where the source information lives
pub struct EtaSource {
    pub name: FileId,
    pub source: Rc<str>,
}

/// Stores the source code and names for all files being compiled, to be indexed with FileId
pub struct SourceManager {
    /// Storage: EtaSource(File Name, Source Content)
    sources: HashMap<FileId, EtaSource>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    pub fn sources(&self) -> impl Iterator<Item = &EtaSource> {
        self.sources.values()
    }

    /// Add a new source file to the manager.
    pub fn add(&mut self, name: impl Into<String>, src: Rc<str>) -> FileId {
        let name = name.into();

        let id = FileId(name.clone());
        self.sources
            .insert(FileId(name.clone()), EtaSource { name: FileId(name), source: src });

        id
    }

    /// id -> Get a new (rc) pointer to the source str
    pub fn get_source_str(&self, id: &FileId) -> Option<Rc<str>> {
        self.sources.get(id).map(|s| s.source.clone())
    }
}
