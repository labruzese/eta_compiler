//! S-expression `Display` for the AST. The exact shape here is relied on by
//! parser tests, so keep node renderings stable.

#![allow(clippy::wildcard_imports)]
use super::*;
use pretty::{Doc, RcDoc};
use std::fmt;

const WIDTH: usize = 80;
const INDENT: isize = 1;

/// Turn any `Display` value into a one-line doc.
fn atom<T: fmt::Display>(x: T) -> RcDoc<'static, ()> {
    RcDoc::text(format!("{x}"))
}

trait ToDoc {
    fn to_doc(&self) -> RcDoc<'static, ()>;
}

/// Build a single doc node:
///   d!("keyword")   → `RcDoc::text("keyword`")
///   d!(@ expr)      → atom(&expr)       (Display-based leaf)
///   d!(expr)        → `expr.to_doc()`     (recursive descent)
macro_rules! d {
    (@$e:expr) => { atom(&$e) };
    ($s:literal) => { RcDoc::text($s) };
    ($e:expr) => { ($e).to_doc() };
}

/// Map an iterable into a doc iterator (for passing to `parens`).
macro_rules! docs {
    ($iter:expr) => { ($iter).iter().map(|x| x.to_doc()) };
}

macro_rules! impl_display {
    ($($t:ty),* $(,)?) => {
        $(
            impl fmt::Display for $t {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.to_doc().render_fmt(WIDTH, f)
                }
            }
        )*
    }
}

impl_display!(
    Program, Interface, InterfaceItem, Use, Definition, MethodDecl, Method, GlobDecl, Value, Decl, Type, Block, Stmt,
    Target, LValue, ProcCall, Expr, Lit, ArrLit,
);

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.sym)
    }
}

/// Wrap docs in parens: inline if short, otherwise break and indent.
fn parens<I>(items: I) -> RcDoc<'static, ()>
where
    I: IntoIterator<Item = RcDoc<'static, ()>>,
    I::IntoIter: DoubleEndedIterator,
{
    d!("(")
        .append(RcDoc::intersperse(items, Doc::line()).nest(INDENT).group())
        .append(d!(")"))
}

impl ToDoc for Program {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens([
            parens(docs!(self.uses)),
            parens(docs!(self.definitions)),
        ])
    }
}

impl ToDoc for Interface {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens([parens(docs!(self.items))])
    }
}

impl ToDoc for InterfaceItem {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match &self.kind {
            InterfaceItemKind::MethodDecl(d) => d.to_doc(),
            InterfaceItemKind::Error => d!("Error"),
        }
    }
}

impl ToDoc for Use {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens([d!("use"), d!(@self.id)])
    }
}

impl ToDoc for Definition {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match &self.kind {
            DefinitionKind::Method(m) => m.to_doc(),
            DefinitionKind::GlobDecl(g) => g.to_doc(),
            DefinitionKind::Error => d!("Error"),
        }
    }
}

impl ToDoc for MethodDecl {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens([d!(@self.id), parens(docs!(self.params)), parens(docs!(self.ret_types))])
    }
}

impl ToDoc for Method {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens([
            d!(@self.id),
            parens(docs!(self.params)),
            parens(docs!(self.ret_types)),
            self.body.to_doc(),
        ])
    }
}

impl ToDoc for GlobDecl {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        let mut items = vec![d!(":global"), d!(@self.id), d!(self.typ)];
        if let Some(v) = &self.val {
            items.push(v.to_doc());
        }
        parens(items)
    }
}

impl ToDoc for Value {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match &self.kind {
            ValueKind::Int(i) => atom(i),
            ValueKind::Bool(b) => atom(b),
        }
    }
}

impl ToDoc for Decl {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens([d!(@self.id), d!(self.typ)])
    }
}

impl ToDoc for Type {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match &self.kind {
            TypeKind::Array { of, size: Some(size) } => parens([d!("[]"), of.to_doc(), size.to_doc()]),
            TypeKind::Array { of, size: None } => parens([d!("[]"), of.to_doc()]),
            TypeKind::Int => d!("int"),
            TypeKind::Bool => d!("bool"),
        }
    }
}

impl ToDoc for Block {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens(docs!(self.stmts))
    }
}

impl ToDoc for Stmt {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match &self.kind {
            StmtKind::Assign { targets, values } => {
                let t = if targets.len() == 1 {
                    targets[0].to_doc()
                } else {
                    parens(docs!(targets))
                };
                let v = if values.len() == 1 {
                    values[0].to_doc()
                } else {
                    parens(docs!(values))
                };
                parens([d!("="), t, v])
            }
            StmtKind::If { cond, then_branch, else_branch } => match else_branch {
                Some(e) => parens([d!("if"), cond.to_doc(), then_branch.to_doc(), e.to_doc()]),
                None => parens([d!("if"), cond.to_doc(), then_branch.to_doc()]),
            },
            StmtKind::While { cond, body } => parens([d!("while"), cond.to_doc(), body.to_doc()]),
            StmtKind::Return { values } => {
                let mut items = vec![d!("return")];
                items.extend(docs!(values));
                parens(items)
            }
            StmtKind::Call(p) => p.to_doc(),
            StmtKind::Block(b) => b.to_doc(),
            StmtKind::Decls(decls) => RcDoc::intersperse(docs!(decls), Doc::line()).group(),
            StmtKind::Error => d!("Error"),
        }
    }
}

impl ToDoc for Target {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Target::LValue(v) => v.to_doc(),
            Target::Decl(d_) => d_.to_doc(),
            Target::Discard(_) => d!("_"),
        }
    }
}

impl ToDoc for LValue {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match &self.kind {
            LValueKind::Index { array, index } => parens([d!("[]"), array.to_doc(), index.to_doc()]),
            LValueKind::Id(id) => d!(@id),
            LValueKind::ProcCall(pc) => pc.to_doc(),
        }
    }
}

impl ToDoc for ProcCall {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        let mut items = vec![d!(@self.id)];
        items.extend(docs!(self.args));
        parens(items)
    }
}

impl ToDoc for Expr {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match &self.kind {
            ExprKind::Id(id) => d!(@id),
            ExprKind::Lit(lit) => lit.to_doc(),
            ExprKind::Index { array, index } => parens([d!("[]"), array.to_doc(), index.to_doc()]),
            ExprKind::Call(pc) => pc.to_doc(),
            ExprKind::Length(e) => parens([d!("length"), e.to_doc()]),
            ExprKind::Unary { op, operand } => parens([d!(@op.node), operand.to_doc()]),
            ExprKind::Binary { op, lhs, rhs } => parens([d!(@op.node), lhs.to_doc(), rhs.to_doc()]),
            ExprKind::Error => d!("Error"),
        }
    }
}

impl ToDoc for Lit {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Lit::Int(i) => atom(i),
            Lit::Bool(b) => atom(b),
            Lit::Char(c) => atom(format!("'{c}'")),
            Lit::Arr(a) => a.to_doc(),
        }
    }
}

impl ToDoc for ArrLit {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            ArrLit::Str(s) => RcDoc::text(format!("\"{}\"", s.escape_default())),
            ArrLit::Array(exprs) => parens(docs!(exprs)),
        }
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
