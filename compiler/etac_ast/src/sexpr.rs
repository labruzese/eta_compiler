//! S-expression rendering backing `#[derive(Sexpr)]`.
//!
//! The derive produces `Sexpr` (and `Display`) impls; this module owns the
//! trait, the `Repr` intermediate form, and the [`SexprCtx`] hook that lets a
//! caller attach arbitrary extra information (spans, types, ids, ...) to any
//! node that carries a [`NodeId`] — without the tree knowing anything about
//! where that information lives.
//!
//! Rendering is a two-step affair so annotations land *inside* a node's
//! parens instead of wrapping them:
//!
//!  1. [`Sexpr::repr`] builds the structural shape of a node as a [`Repr`]
//!     (a bare atom, an open list of parts, or an already-finished doc).
//!  2. [`Sexpr::to_doc`] finishes the shape. For node types (anything with a
//!     `node_id`) the derive overrides `to_doc` to ask the context for an
//!     annotation first, so `(= x 1)` becomes `(= x 1 @3:1-3:6)` and the atom
//!     `x` becomes `(x @3:5)`.
//!
//! `Display` renders through [`Plain`], which never annotates.

use crate::NodeId;
use pretty::RcDoc;
use std::fmt;

pub const WIDTH: usize = 80;
pub const INDENT: isize = 1;

/// The structural shape of a node, before annotations are spliced in.
pub enum Repr {
    /// A bare token (`foo`, `42`, `int`). Annotating an atom wraps it:
    /// `(foo @1:2)`.
    Atom(RcDoc<'static, ()>),
    /// The parts of a parenthesized list, still open so annotations can be
    /// appended before the closing paren.
    List(Vec<RcDoc<'static, ()>>),
    /// An already-finished doc (used by transparent delegation and custom
    /// renderers). Annotating wraps it in parens.
    Raw(RcDoc<'static, ()>),
}

/// Context consulted while printing. 
/// Implement this to attach extra information to nodes; the default annotates nothing.
pub trait SexprCtx {
    /// Extra doc to append inside the parens of the node with this id (or
    /// wrapping it, if the node renders as a bare atom). Return `None` to
    /// leave the node untouched.
    fn annotate(&self, _node_id: NodeId) -> Option<RcDoc<'static, ()>> {
        None
    }
}

/// The no-annotation context; `Display` renders through this.
pub struct Plain;
impl SexprCtx for Plain {}

pub trait Sexpr {
    /// The structural shape of this value.
    fn repr(&self, ctx: &dyn SexprCtx) -> Repr;

    /// The finished doc. The derive overrides this for types with a
    /// `node_id` to splice in `ctx.annotate(id)`.
    fn to_doc(&self, ctx: &dyn SexprCtx) -> RcDoc<'static, ()> {
        finish(self.repr(ctx), None)
    }
}

// doc helpers (used by generated code)

/// Turn any `Display` value into a one-line doc.
pub fn atom<T: fmt::Display>(x: T) -> RcDoc<'static, ()> {
    RcDoc::text(format!("{x}"))
}

/// Wrap docs in parens: inline if short, otherwise break and indent.
pub fn parens<I>(items: I) -> RcDoc<'static, ()>
where
    I: IntoIterator<Item = RcDoc<'static, ()>>,
{
    RcDoc::text("(")
        .append(
            RcDoc::intersperse(items, RcDoc::line())
                .nest(INDENT)
                .group(),
        )
        .append(RcDoc::text(")"))
}

/// Close a [`Repr`], splicing in an optional annotation.
pub fn finish(repr: Repr, ann: Option<RcDoc<'static, ()>>) -> RcDoc<'static, ()> {
    match (repr, ann) {
        (Repr::Atom(d) | Repr::Raw(d), None) => d,
        (Repr::Atom(d) | Repr::Raw(d), Some(a)) => parens([d, a]),
        (Repr::List(items), None) => parens(items),
        (Repr::List(mut items), Some(a)) => {
            items.push(a);
            parens(items)
        }
    }
}

// ---- blanket / leaf impls ----

impl<T: Sexpr + ?Sized> Sexpr for Box<T> {
    fn repr(&self, ctx: &dyn SexprCtx) -> Repr {
        (**self).repr(ctx)
    }
    fn to_doc(&self, ctx: &dyn SexprCtx) -> RcDoc<'static, ()> {
        (**self).to_doc(ctx)
    }
}

impl<T: Sexpr + ?Sized> Sexpr for &T {
    fn repr(&self, ctx: &dyn SexprCtx) -> Repr {
        (**self).repr(ctx)
    }
    fn to_doc(&self, ctx: &dyn SexprCtx) -> RcDoc<'static, ()> {
        (**self).to_doc(ctx)
    }
}

impl Sexpr for str {
    fn repr(&self, _: &dyn SexprCtx) -> Repr {
        Repr::Atom(RcDoc::text(self.to_owned()))
    }
}

impl Sexpr for String {
    fn repr(&self, _: &dyn SexprCtx) -> Repr {
        Repr::Atom(RcDoc::text(self.clone()))
    }
}

macro_rules! atom_impls {
    ($($t:ty),* $(,)?) => {
        $(
            impl Sexpr for $t {
                fn repr(&self, _: &dyn SexprCtx) -> Repr {
                    Repr::Atom(atom(self))
                }
            }
        )*
    };
}
atom_impls!(bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
