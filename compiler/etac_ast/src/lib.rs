//! Abstract syntax tree.
//!
//! Span placement: every node is *either* a struct with its own
//! `span: Span` field, *or* an enum of the form `Foo { span, kind: FooKind }`.
//! So `node.span` is always available — including the `Error` recovery variants.
//! Small payloads are inlined as struct-variants (`ExprKind::Binary { .. }`),
//! so the only struct types left are genuine nodes. The two types that *only*
//! ever appear inside a spanned `Expr` (`Lit`, `ArrLit`) are deliberately
//! span-free and inherit their location from the enclosing `Expr`.

use etac_span::Span;
use etac_derive_spanned::Spanned;

mod printer;

pub type Id = String;

/// Uniform span access for any node that carries one.
pub trait Spanned {
    fn span(&self) -> Span;
}

// ---- Identifiers ----

#[derive(Debug, Clone, Spanned)]
pub struct Ident {
    pub span: Span,
    pub sym: Id,
}

// ---- Top level ----

#[derive(Debug, Clone)]
pub struct Program {
    pub uses: Vec<Use>,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, Clone)]
pub struct Interface {
    pub items: Vec<InterfaceItem>,
}

#[derive(Debug, Clone, Spanned)]
pub struct Use {
    pub span: Span,
    pub id: Ident,
}

#[derive(Debug, Clone, Spanned)]
pub struct Definition {
    pub span: Span,
    pub kind: DefinitionKind,
}

#[derive(Debug, Clone)]
pub enum DefinitionKind {
    Method(Method),
    GlobDecl(GlobDecl),
    Error,
}

#[derive(Debug, Clone, Spanned)]
pub struct InterfaceItem {
    pub span: Span,
    pub kind: InterfaceItemKind,
}

#[derive(Debug, Clone)]
pub enum InterfaceItemKind {
    Decl(MethodDecl),
    Error,
}

// ---- Methods & globals ----

#[derive(Debug, Clone, Spanned)]
pub struct MethodDecl {
    pub span: Span,
    pub id: Ident,
    pub params: Vec<Decl>,
    pub ret_types: Vec<Type>,
}

#[derive(Debug, Clone, Spanned)]
pub struct Method {
    pub span: Span,
    pub id: Ident,
    pub params: Vec<Decl>,
    pub ret_types: Vec<Type>,
    pub body: Block,
}

#[derive(Debug, Clone, Spanned)]
pub struct GlobDecl {
    pub span: Span,
    pub id: Ident,
    pub typ: Type,
    pub val: Option<Value>,
}

#[derive(Debug, Clone, Spanned)]
pub struct Value {
    pub span: Span,
    pub kind: ValueKind,
}

#[derive(Debug, Clone)]
pub enum ValueKind {
    Int(i128),
    Bool(bool),
}

#[derive(Debug, Clone, Spanned)]
pub struct Decl {
    pub span: Span,
    pub id: Ident,
    pub typ: Type,
}

// ---- Types ----

#[derive(Debug, Clone, Spanned)]
pub struct Type {
    pub span: Span,
    pub kind: TypeKind,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    SizedArray { of: Box<Type>, size: Box<Expr> },
    UnsizedArray { of: Box<Type> },
    Int,
    Bool,
}

// ---- Blocks & statements ----

#[derive(Debug, Clone, Spanned)]
pub struct Block {
    pub span: Span,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, Spanned)]
pub struct Stmt {
    pub span: Span,
    pub kind: StmtKind,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Assign { targets: Vec<Target>, values: Vec<Expr> },
    If { cond: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>> },
    While { cond: Expr, body: Box<Stmt> },
    Return { values: Vec<Expr> },
    Call(ProcCall),
    Block(Block),
    Decls(Vec<Decl>),
    Error,
}

// ---- Targets & lvalues ----

#[derive(Debug, Clone, Spanned)]
pub struct Target {
    pub span: Span,
    pub kind: TargetKind,
}

#[derive(Debug, Clone)]
pub enum TargetKind {
    LValue(LValue),
    Decl(Decl),
    Discard,
}

impl Target {
    /// Wrap a declaration as an assignment target, inheriting its span.
    pub fn from_decl(d: Decl) -> Target {
        Target { span: d.span, kind: TargetKind::Decl(d) }
    }
}

#[derive(Debug, Clone, Spanned)]
pub struct LValue {
    pub span: Span,
    pub kind: LValueKind,
}

#[derive(Debug, Clone)]
pub enum LValueKind {
    Index { of: Box<LValue>, index: Box<Expr> },
    Id(Ident),
    ProcCall(ProcCall),
}

// ---- Calls ----

#[derive(Debug, Clone, Spanned)]
pub struct ProcCall {
    pub span: Span,
    pub id: Ident,
    pub args: Vec<Expr>,
}

// ---- Expressions ----

#[derive(Debug, Clone, Spanned)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Id(Ident),
    Lit(Lit),
    Index { array: Box<Expr>, index: Box<Expr> },
    Call(ProcCall),
    Length(Box<Expr>),
    Unary { op: UOp, op_span: Span, operand: Box<Expr> },
    Binary { op: BinOp, op_span: Span, lhs: Box<Expr>, rhs: Box<Expr> },
    Error,
}

#[derive(Debug, Clone, Copy)]
pub enum UOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    HighMul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

// ---- Literals (span-free; inherit from the enclosing Expr) ----

#[derive(Debug, Clone)]
pub enum Lit {
    Int(i128),
    Bool(bool),
    Char(char),
    Arr(ArrLit),
}

#[derive(Debug, Clone)]
pub enum ArrLit {
    Str(String),
    Array(Vec<Expr>),
}
