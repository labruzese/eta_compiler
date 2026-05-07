use super::*;
use pretty::{Doc, RcDoc};
use std::fmt::{self, Display};

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
///   d!("keyword")   → RcDoc::text("keyword")
///   d!(@ expr)      → atom(&expr)       (Display-based leaf)
///   d!(expr)        → expr.to_doc()     (recursive descent)
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
    Program, Interface, Use, Definition, MethodDecl, Method, GlobDecl, Value, Decl, Type, Block, Stmt, Assignment,
    AssignLeft, LValue, IfStmt, WhileStmt, ReturnStmt, ProcCall, Expr, Lit, ArrLit,
);

impl<T: Display> Display for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.node, f)
    }
}

impl<T: ToDoc> ToDoc for Spanned<T> {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        d!(self.node)
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
            parens(docs!(self.definitions))
        ])
    }
}

impl ToDoc for Interface {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens([
            parens(docs!(self.method_decls))
        ])
    }
}

impl ToDoc for Use {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens([
            d!("use"),
            d!(@self.id)
        ])
    }
}

impl ToDoc for Definition {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Definition::Method(m) => d!(m),
            Definition::GlobDecl(g) => d!(g),
            Definition::Error => d!("Error"),
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
            d!(self.body),
        ])
    }
}

impl ToDoc for GlobDecl {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        let mut items = vec![d!(":global"), d!(@self.id), d!(self.typ)];
        if let Some(v) = &self.val {
            items.push(d!(v));
        }
        parens(items)
    }
}

impl ToDoc for Value {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Value::IntLit(i) => d!(@i),
            Value::BoolLit(b) => d!(@b),
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
        match self {
            Type::SizedArray(s) => parens([d!("[]"), d!(s.node.of), d!(s.node.size)]),
            Type::UnsizedArray(u) => parens([d!("[]"), d!(u.node.of)]),
            Type::Int(_) => d!("int"),
            Type::Bool(_) => d!("bool"),
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
        match self {
            Stmt::Assignment(a) => d!(a),
            Stmt::IfStmt(i) => d!(i),
            Stmt::WhileStmt(w) => d!(w),
            Stmt::ReturnStmt(r) => d!(r),
            Stmt::ProcCall(p) => d!(p),
            Stmt::Block(b) => d!(b),
            Stmt::Decls(decls) => RcDoc::intersperse(docs!(decls), Doc::line()).group(),
            Stmt::Error => d!("Error"),
        }
    }
}

impl ToDoc for Assignment {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        let t = if self.targets.len() == 1 {
            d!(self.targets[0])
        } else {
            parens(docs!(self.targets))
        };
        let v = if self.values.len() == 1 {
            d!(self.values[0])
        } else {
            parens(docs!(self.values))
        };
        parens([d!("="), t, v])
    }
}

impl ToDoc for AssignLeft {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            AssignLeft::LValue(v) => d!(v),
            AssignLeft::Decl(d_) => d_.to_doc(),
            AssignLeft::Ignore => d!("_"),
        }
    }
}

impl ToDoc for LValue {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            LValue::Index(s) => parens([d!("[]"), d!(s.node.of), d!(s.node.index)]),
            LValue::Id(id) => d!(@ id),
            LValue::ProcCall(pc) => d!(pc),
        }
    }
}

impl ToDoc for IfStmt {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match &self.else_branch {
            Some(e) => parens([d!("if"), d!(self.cond), d!(self.then_branch), d!(e)]),
            None => parens([d!("if"), d!(self.cond), d!(self.then_branch)]),
        }
    }
}

impl ToDoc for WhileStmt {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        parens([d!("while"), d!(self.cond), d!(self.body)])
    }
}

impl ToDoc for ReturnStmt {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        let mut items = vec![d!("return")];
        items.extend(docs!(self.values));
        parens(items)
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
        match self {
            Expr::Id(id) => d!(@id),
            Expr::Lit(lit) => d!(lit),
            Expr::Index(s) => parens([d!("[]"), d!(s.node.array), d!(s.node.index)]),
            Expr::Call(pc) => d!(pc),
            Expr::Length(e) => parens([d!("length"), d!(e.node)]),
            Expr::Unary(s) => parens([d!(@s.node.op), d!(s.node.expr)]),
            Expr::Binary(s) => parens([d!(@s.node.op), d!(s.node.left), d!(s.node.right)]),
        }
    }
}

impl ToDoc for Lit {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Lit::IntLit(i) => d!(@i),
            Lit::BoolLit(b) => d!(@b),
            Lit::CharLit(c) => atom(format!("\'{c}\'")),
            Lit::ArrLit(a) => d!(a),
        }
    }
}

impl ToDoc for ArrLit {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            ArrLit::StringLit(s) => RcDoc::text(format!("\"{}\"", s.node.escape_default())),
            ArrLit::Array(exprs) => parens(docs!(exprs)),
        }
    }
}

impl fmt::Display for UOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UOp::Neg => "-",
            UOp::Not => "!",
        })
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::HighMul => "*>>",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Eq => "==",
            BinOp::Neq => "!=",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::Le => "<=",
            BinOp::Ge => ">=",
            BinOp::And => "&",
            BinOp::Or => "|",
        })
    }
}
