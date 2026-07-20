pub type NodeId<T> = id_arena::Id<T>;

/// This is a concrete (struct) node, enums don't count
pub trait AstNode<T>: std::fmt::Debug {
    fn node_id(&self) -> NodeId<T>;
}
