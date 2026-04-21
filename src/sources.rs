use std::fmt;
use std::rc::Rc;

mod span;
pub use span::*;

/// A file identifier. The inner `Rc<str>` is the filename/path — cheap to
/// clone (refcount bump) and doubles as ariadne's cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileId(Rc<str>);

impl FileId {
    pub fn new(name: impl Into<Rc<str>>) -> Self { Self(name.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

/// Loader invoked by ariadne on cache miss. Default: read from disk,
/// interpreting the `FileId` as a path. Returns `Rc<str>` so the source
/// text is never copied again — ariadne's `Source<Rc<str>>` and your
/// lexer/parser can share the same allocation.
pub type SourceLoader = Box<dyn FnMut(&FileId) -> Result<Rc<str>, std::io::Error>>;

pub type Sources = ariadne::FnCache<FileId, SourceLoader, Rc<str>>;

/// A `Sources` that lazily reads files from disk on first access.
pub fn sources_from_disk() -> Sources {
    ariadne::FnCache::new(Box::new(|id: &FileId| {
        std::fs::read_to_string(id.as_str()).map(Rc::from)
    }))
}
