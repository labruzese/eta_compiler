use etac_ast::id::NodeId;

use crate::sources::Span;

#[derive(Default)]
pub struct AstArena {
    pub spans: SpanMap,
}

#[derive(Default)]
pub struct SpanMap {
    map: Vec<Span>,
}

impl SpanMap {
    pub fn span<T>(&self, node: NodeId<T>) -> Span {
        *self.map
            .get(node.index())
            .expect("nodes always allocate with a span")
    }
}
