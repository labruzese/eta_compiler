//! Node identity.
//!
//! A [`NodeId`] is minted by the compilation cache's span allocator, which
//! records the node's span at the same moment — so an id without a span
//! cannot exist. This crate only carries ids; it never resolves them.

/// Stable identifier for an AST node. Do not construct directly: the cache's
/// span allocator is the only legitimate source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

impl NodeId {
    /// Placeholder for synthesized nodes before real ids are assigned.
    pub const DUMMY: NodeId = NodeId(u32::MAX);

    #[must_use]
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Reserved for the id allocator. Constructing ids anywhere else breaks
    /// the id-implies-recorded-span invariant.
    #[doc(hidden)]
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        NodeId(raw)
    }
}

pub trait AstNode {
    fn node_id(&self) -> NodeId;
}
