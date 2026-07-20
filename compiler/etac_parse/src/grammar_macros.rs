/// Special syntax to allocate an AST node with a span.
/// Returns ownership of the ast node.
macro_rules! alloc {
    ($ty:ident::new($args:expr) @ $span:expr) => {
        <$ty>::new(state.cache.alloc_span(Span::new($span.start, $span.end), $args))
    };
}
