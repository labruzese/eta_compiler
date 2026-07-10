mod sources;
pub use sources::*;

mod ariadne_compat;
pub use ariadne_compat::*;

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u32);
pub type SourceId = FileId;
pub type InterfaceId = FileId;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    /// Placeholder span for synthesized nodes. Should never reach a diagnostic.
    pub const DUMMY: Span = Span { lo: 0, hi: 0 };

    pub fn new(lo: impl Into<u32>, hi: impl Into<u32>) -> Self {
        Self {
            lo: lo.into(),
            hi: hi.into(),
        }
    }

    pub fn to(self, other: Span) -> Span {
        Span {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }

    #[must_use]
    pub fn len(self) -> u32 {
        self.hi - self.lo
    }
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.lo == self.hi
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.lo, self.hi)
    }
}
