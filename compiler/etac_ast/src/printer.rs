//! What's left of the S-expression printer now that `#[derive(Sexpr)]`
//! generates the structural rendering: the handful of shapes too bespoke for
//! attributes (`sexpr(with = ...)` targets), the `Leaf` impl, and an example
//! annotation context that prints side-table spans next to each node.
//!
//! The exact `Display` shapes are relied on by parser tests; `Display` always
//! renders through [`Plain`], so derived output stays stable no matter what
//! contexts exist.

use super::*;
use crate::sexpr::{atom, finish, Plain, Repr, Sexpr, SexprCtx, WIDTH};
use pretty::RcDoc;
use std::fmt;

// custom renderers (`#[sexpr(with = "printer::...")]`)

pub(super) fn decls_repr(decls: &[Decl], ctx: &dyn SexprCtx) -> Repr {
    Repr::Raw(
        RcDoc::intersperse(decls.iter().map(|d| d.to_doc(ctx)), RcDoc::line()).group(),
    )
}

pub(super) fn char_repr(c: &char, _: &dyn SexprCtx) -> Repr {
    Repr::Atom(atom(format!("'{c}'")))
}

pub(super) fn str_repr(s: &str, _: &dyn SexprCtx) -> Repr {
    Repr::Atom(RcDoc::text(format!("\"{}\"", s.escape_default())))
}


impl<T: fmt::Display> Sexpr for Leaf<T> {
    fn repr(&self, _: &dyn SexprCtx) -> Repr {
        Repr::Atom(atom(&self.node))
    }

    fn to_doc(&self, ctx: &dyn SexprCtx) -> RcDoc<'static, ()> {
        finish(self.repr(ctx), ctx.annotate(self.node_id))
    }
}

impl<T: fmt::Display> fmt::Display for Leaf<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_doc(&Plain).render_fmt(WIDTH, f)
    }
}

impl fmt::Display for UOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub mod spans {
    use super::*;

    /// Annotates every node that has a recorded span with `@line:col-line:col`.
    ///
    /// Generic over the lookup so it works with the real compilation cache:
    /// `program.sexpr_with(&SpanCtx::new(|id| cache.span(id)))`.
    pub struct SpanCtx<F>(F);

    impl<F> SpanCtx<F>
    where
        F: Fn(NodeId) -> Option<String>,
    {
        pub fn new(lookup: F) -> Self {
            SpanCtx(lookup)
        }
    }

    impl<F> SexprCtx for SpanCtx<F>
    where
        F: Fn(NodeId) -> Option<String>,
    {
        fn annotate(&self, node_id: NodeId) -> Option<RcDoc<'static, ()>> {
            (self.0)(node_id).map(|span| RcDoc::text(format!("@{span}")))
        }
    }
}
