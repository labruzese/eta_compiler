//! Node identity and the span side table.
//!
//! Allocation and recording are fused: [`SpanTable::alloc`] both mints the id
//! and stores its span, so an id without a span cannot exist.

use etac_span::Span;

/// Stable identifier for an AST node. Allocated by [`SpanTable::alloc`]; do not
/// construct directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

impl NodeId {
    /// Placeholder for synthesized nodes before real ids are assigned.
    /// [`SpanTable::get`] maps it to [`Span::DUMMY`].
    pub const DUMMY: NodeId = NodeId(u32::MAX);

    #[must_use]
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

pub trait AstNode {
    fn node_id(&self) -> NodeId;
}

/// Side table mapping every [`NodeId`] to its [`Span`]; also the id allocator.
#[derive(Debug, Default)]
pub struct SpanTable {
    spans: Vec<Span>,
}

impl SpanTable {
    #[must_use]
    pub fn new() -> Self {
        SpanTable { spans: Vec::new() }
    }

    /// fresh id with `span` recorded for it.
    pub fn alloc(&mut self, span: Span) -> NodeId {
        let idx = self.spans.len();
        assert!(idx < u32::MAX as usize, "NodeId space exhausted");
        #[allow(clippy::cast_possible_truncation)]
        let id = NodeId(idx as u32);
        self.spans.push(span);
        id
    }

    /// Fresh id sharing the span already recorded for `of`. For wrapper nodes
    /// and reinterpretations that cover the same source text.
    pub fn dup(&mut self, of: NodeId) -> NodeId {
        let span = self.get(of);
        self.alloc(span)
    }

    /// The span recorded for `id`.
    ///
    /// [`NodeId::DUMMY`] maps to [`Span::DUMMY`]. Any other id this map did
    /// not allocate is a logic error (an id from a foreign map) and panics.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Span {
        if id == NodeId::DUMMY {
            return Span::DUMMY;
        }
        self.spans[id.0 as usize]
    }

    /// Convenience for `self.get(node.node_id())`.
    #[must_use]
    pub fn span_of(&self, node: &impl AstNode) -> Span {
        self.get(node.node_id())
    }

    /// Number of ids allocated so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.spans.len()
    }
}
