use std::sync::RwLock;

use etac_ast::NodeId;

use crate::Span;

/// Maps every [`NodeId`] to its [`Span`]; also the id allocator.
///
/// Allocation and recording are fused: [`alloc`](SpanTable::alloc) both mints
/// the id and stores its span, so an id without a span cannot exist. One table
/// serves the whole compilation, so ids are unique across all trees.
#[derive(Debug, Default)]
pub(crate) struct SpanTable {
    spans: RwLock<Vec<Span>>,
}

impl SpanTable {
    pub(crate) fn alloc(&self, span: Span) -> NodeId {
        let mut spans = self.spans.write().expect("span table poisoned");
        let idx = spans.len();
        assert!(idx < u32::MAX as usize, "NodeId space exhausted");
        #[allow(clippy::cast_possible_truncation)]
        let id = NodeId::from_raw(idx as u32);
        spans.push(span);
        id
    }

    /// The span recorded for `id`.
    ///
    /// [`NodeId::DUMMY`] maps to [`Span::DUMMY`]. Any other id this table did
    /// not allocate is a logic error and panics.
    pub(crate) fn get(&self, id: NodeId) -> Span {
        if id == NodeId::DUMMY {
            return Span::DUMMY;
        }
        self.spans.read().expect("span table poisoned")[id.as_u32() as usize]
    }
}
