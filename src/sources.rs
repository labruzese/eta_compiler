use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::{self, Display};
use std::path::Path;
use std::rc::Rc;

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
    pub name: String,
    pub source: Rc<str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtaSpan<'fid> {
    pub file_id: &'fid FileId,
    pub range: std::ops::Range<usize>,
}

impl<'fid> From<(&'fid FileId, std::ops::Range<usize>)> for EtaSpan<'fid> {
    fn from(value: (&'fid FileId, std::ops::Range<usize>)) -> Self {
        EtaSpan {
            file_id: value.0,
            range: value.1,
        }
    }
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

    pub fn ids(&self) -> impl Iterator<Item = &FileId> {
        self.sources.keys()
    }

    /// Add a new source file to the manager.
    pub fn add(&mut self, name: impl Into<String>, src: Rc<str>) -> FileId {
        let name = name.into();

        let id = FileId(name.clone());
        self.sources
            .insert(FileId(name.clone()), EtaSource { name, source: src });

        id
    }

    /// id -> Borrow the file name
    pub fn get_file_name(&self, id: &FileId) -> Option<&str> {
        self.sources.get(id).map(|s| {
            if let Some(n) = Path::new(s.name.as_str())
                .file_stem()
                .and_then(|x| x.to_str())
            {
                n
            } else {
                s.name.as_str()
            }
        })
    }

    /// id -> Get a new (rc) pointer to the source str
    pub fn get_source(&self, id: &FileId) -> Option<Rc<str>> {
        self.sources.get(id).map(|s| s.source.clone())
    }
}
