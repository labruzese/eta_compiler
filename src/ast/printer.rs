use super::*;
use pretty::{Doc, RcDoc};
use std::fmt;

const WIDTH: usize = 80;
const INDENT: isize = 2;

/// Turn any `Display` value into a one-line doc.
fn atom<T: fmt::Display>(x: T) -> RcDoc<'static, ()> {
    RcDoc::text(format!("{x}"))
}

/// Wrap docs in parens: inline if short, otherwise break and indent.
fn parens<I>(items: I) -> RcDoc<'static, ()>
where
    I: IntoIterator<Item = RcDoc<'static, ()>>,
    I::IntoIter: DoubleEndedIterator,
{
    RcDoc::text("(")
        .append(RcDoc::intersperse(items, Doc::line()).nest(INDENT).group())
        .append(RcDoc::text(")"))
}

impl Program {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Program::Prog { uses, definitions } => parens([
                parens(uses.iter().map(Use::to_doc)),
                parens(definitions.iter().map(Definition::to_doc)),
            ]),
        }
    }
}

impl Use {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Use::Id(id) => parens([RcDoc::text("use"), atom(id)]),
        }
    }
}

impl Definition {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Definition::Method(m) => m.to_doc(),
            Definition::GlobDecl(g) => g.to_doc(),
        }
    }
}

impl Method {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Method::Method {
                id,
                params,
                ret_types,
                body,
            } => parens([
                atom(id),
                parens(params.iter().map(Decl::to_doc)),
                parens(ret_types.iter().map(Type::to_doc)),
                body.to_doc(),
            ]),
        }
    }
}

impl GlobDecl {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            GlobDecl::GlobDecl { id, typ, val } => {
                let mut items = vec![RcDoc::text(":global"), atom(id), typ.to_doc()];
                if let Some(v) = val {
                    items.push(v.to_doc());
                }
                parens(items)
            }
        }
    }
}

impl Value {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Value::IntLit(i) => atom(i),
            Value::BoolLit(b) => atom(b),
        }
    }
}

impl Decl {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Decl::Decl { id, typ } => parens([atom(id), typ.to_doc()]),
        }
    }
}

impl Type {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Type::SizedArray { of, size } => parens([RcDoc::text("[]"), of.to_doc(), atom(size)]),
            Type::UnsizedArray { of } => parens([RcDoc::text("[]"), of.to_doc()]),
            Type::Int => RcDoc::text("int"),
            Type::Bool => RcDoc::text("bool"),
        }
    }
}

impl Block {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Block::Block { stmts } => parens(stmts.iter().map(Stmt::to_doc)),
        }
    }
}

impl Stmt {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Stmt::Assignment(a) => a.to_doc(),
            Stmt::IfStmt(i) => i.to_doc(),
            Stmt::WhileStmt(w) => w.to_doc(),
            Stmt::ReturnStmt(r) => r.to_doc(),
            Stmt::ProcCall(p) => p.to_doc(),
            Stmt::Block(b) => b.to_doc(),
            Stmt::Decls { decls } => {
                RcDoc::intersperse(decls.iter().map(Decl::to_doc), Doc::line()).group()
            }
        }
    }
}

impl Assignment {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Assignment::Assignment { targets, values } => {
                let t = if targets.len() == 1 {
                    targets[0].to_doc()
                } else {
                    parens(targets.iter().map(AssignLeft::to_doc))
                };

                let v = if values.len() == 1 {
                    values[0].to_doc()
                } else {
                    parens(values.iter().map(Expr::to_doc))
                };

                parens([RcDoc::text("="), t, v])
            }
        }
    }
}

impl AssignLeft {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            AssignLeft::Var(v) => v.to_doc(),
            AssignLeft::Decl(d) => d.to_doc(),
            AssignLeft::Ignore => RcDoc::text("_"),
        }
    }
}

impl Var {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Var::Index { of, index } => parens([RcDoc::text("[]"), of.to_doc(), index.to_doc()]),
            Var::Id(id) => atom(id),
        }
    }
}

impl IfStmt {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            IfStmt::IfStmt {
                cond,
                then_branch,
                else_branch,
            } => match else_branch {
                Some(e) => 
                    parens([
                        RcDoc::text("if"),
                        cond.to_doc(),
                        then_branch.to_doc(),
                        e.to_doc()]),
                None => 
                    parens([
                        RcDoc::text("if"),
                        cond.to_doc(),
                        then_branch.to_doc()]),

            } 
        }
    }
}

impl WhileStmt {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            WhileStmt::WhileStmt { cond, body } => {
                parens([RcDoc::text("while"), cond.to_doc(), body.to_doc()])
            }
        }
    }
}

impl ReturnStmt {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            ReturnStmt::ReturnStmt { values } => {
                let mut items = vec![RcDoc::text("return")];
                items.extend(values.iter().map(Expr::to_doc));
                parens(items)
            }
        }
    }
}

impl ProcCall {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            ProcCall::ProcCall { id, args } => {
                let mut items = vec![atom(id)];
                items.extend(args.iter().map(Expr::to_doc));
                parens(items)
            }
        }
    }
}

impl Expr {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Expr::Id(id) => atom(id),
            Expr::Lit(lit) => lit.to_doc(),
            Expr::Index { array, index } => {
                parens([RcDoc::text("[]"), array.to_doc(), index.to_doc()])
            }
            Expr::Call(pc) => pc.to_doc(),
            Expr::Length(e) => parens([RcDoc::text("length"), e.to_doc()]),
            Expr::Unary { op, expr } => parens([atom(op), expr.to_doc()]),
            Expr::Binary { op, left, right } => parens([atom(op), left.to_doc(), right.to_doc()]),
        }
    }
}

impl Lit {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Lit::IntLit(i) => atom(i),
            Lit::BoolLit(b) => atom(b),
            Lit::CharLit(c) => atom(format!("\'{c}\'")),
            Lit::ArrLit(a) => a.to_doc(),
        }
    }
}

impl ArrLit {
    fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            ArrLit::StringLit(s) => RcDoc::text(format!("\"{}\"", s.escape_default())),
            ArrLit::Array(exprs) => parens(exprs.iter().map(Expr::to_doc)),
        }
    }
}

// UOp and BinOp are always single atoms, so plain Display is cleanest.
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
            BinOp::Eq => "=",
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

// Re-expose Display on every AST node by delegating to to_doc.
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
    Program, Use, Definition, Method, GlobDecl, Value, Decl, Type, Block, Stmt, Assignment,
    AssignLeft, Var, IfStmt, WhileStmt, ReturnStmt, ProcCall, Expr, Lit, ArrLit,
);
