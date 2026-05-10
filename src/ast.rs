/// ┌─────────────────────────── AST Structure ─────────────────────────────┐
/// │                                                                       │
/// │  Program                            Interface                         │
/// │  ├── Use*                           └── MethodDecl*                   │
/// │  │   └── id                             ├── id                        │
/// │  └── Definition*                        ├── params: Decl*             │
/// │      ├── Method                         └── ret_types: Type*          │
/// │      │   ├── id                                                       │
/// │      │   ├── params: Decl*          Type                              │
/// │      │   │   ├── id                 ├── Int                           │
/// │      │   │   └── typ: Type          ├── Bool                          │
/// │      │   ├── ret_types: Type*       ├── SizedArray(Type, Expr)        │
/// │      │   └── body: Block            └── UnsizedArray(Type)            │
/// │      └── GlobDecl                                                     │
/// │          ├── id                     Expr                              │
/// │          ├── typ: Type              ├── Id                            │
/// │          └── val?: Value            ├── Lit                           │
/// │              ├── IntLit             │   ├── IntLit                    │
/// │              └── BoolLit            │   ├── BoolLit                   │
/// │                                     │   ├── CharLit                   │
/// │  Block                              │   └── ArrLit                    │
/// │  └── Stmt*                          │       ├── StringLit             │
/// │      ├── Assignment                 │       └── Array(Expr*)          │
/// │      │   ├── targets: AssignLeft*   ├── Index(Expr, Expr)             │
/// │      │   │   ├── LValue             ├── Call(ProcCall)                │
/// │      │   │   │   ├── Id             ├── Length(Expr)                  │
/// │      │   │   │   ├── Index          ├── Unary(UOp, Expr)              │
/// │      │   │   │   └── ProcCall       └── Binary(BinOp, Expr, Expr)     │
/// │      │   │   ├── Decl                                                 │
/// │      │   │   └── Ignore             UOp: Neg | Not                    │
/// │      │   └── values: Expr*          BinOp: Add | Sub | Mul |          │
/// │      ├── IfStmt                        HighMul | Div | Mod |          │
/// │      │   ├── cond: Expr                     Eq | Neq | Lt  | Gt |     │
/// │      │   ├── then: Stmt                     Le | Ge | And | Or        │
/// │      │   └── else?: Stmt                                              │
/// │      ├── WhileStmt                                                    │
/// │      │   ├── cond: Expr                                               │
/// │      │   └── body: Stmt                                               │
/// │      ├── ReturnStmt(Expr*)                                            │
/// │      ├── ProcCall(id, Expr*)                                          │
/// │      ├── Block                                                        │
/// │      └── Decls(Decl*)                                                 │
/// └───────────────────────────────────────────────────────────────────────┘

use crate::sources::span::EtaSpan;

/// helper macro for grammar.lalrpop
/// file_id will be in scope, this will wrap a node with a span
#[macro_export]
macro_rules! sp {
    ($fid:expr, $l:expr, $node:expr, $r:expr) => {
        Spanned::new(EtaSpan::new($fid.clone(),$l, $r), $node)
    };
}

mod printer;

#[derive(Debug, Clone)]
/// Wraps an AST node with it's span, both file and location
pub struct Spanned<T> {
    pub span: EtaSpan,
    pub node: T,
}

impl<T> Spanned<T> {
    pub fn new(span: EtaSpan, node: T) -> Self {
        Self { span, node }
    }
}

pub type Id = String;
pub type IntLit = i128;
pub type BoolLit = bool;
pub type CharLit = char;

#[derive(Debug, Clone)]
pub struct Program {
    pub uses: Vec<Spanned<Use>>,
    pub definitions: Vec<Definition>, 
}

pub struct Interface {
    pub method_decls: Vec<Spanned<MethodDecl>>
}

#[derive(Debug, Clone)]
pub struct Use {
    pub id: Spanned<Id>,
}

#[derive(Debug, Clone)]
pub enum Definition {
    Method(Spanned<Method>),
    GlobDecl(Spanned<GlobDecl>),
    Error,
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub id: Spanned<Id>,
    pub params: Vec<Spanned<Decl>>,
    pub ret_types: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct Method {
    pub id: Spanned<Id>,
    pub params: Vec<Spanned<Decl>>,
    pub ret_types: Vec<Type>,
    pub body: Spanned<Block>,
}

#[derive(Debug, Clone)]
pub struct GlobDecl {
    pub id: Spanned<Id>,
    pub typ: Type,
    pub val: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum Value {
    IntLit(Spanned<IntLit>),
    BoolLit(Spanned<BoolLit>),
}

#[derive(Debug, Clone)]
pub struct Decl {
    pub id: Spanned<Id>,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub enum Type {
    SizedArray(Spanned<SizedArray>),
    UnsizedArray(Spanned<UnsizedArray>),
    Int(EtaSpan),
    Bool(EtaSpan),
}

#[derive(Debug, Clone)]
pub struct SizedArray {
    pub of: Box<Type>,
    pub size: Expr,
}

#[derive(Debug, Clone)]
pub struct UnsizedArray {
    pub of: Box<Type>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assignment(Spanned<Assignment>),
    IfStmt(Spanned<IfStmt>),
    WhileStmt(Spanned<WhileStmt>),
    ReturnStmt(Spanned<ReturnStmt>),
    ProcCall(Spanned<ProcCall>),
    Block(Spanned<Block>),
    Decls(Vec<Spanned<Decl>>),
    Error,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub targets: Vec<Target>,
    pub values: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub enum Target {
    LValue(LValue),
    Decl(Spanned<Decl>),
    Discard(EtaSpan),
}

#[derive(Debug, Clone)]
pub enum LValue {
    Index(Spanned<LValueIndex>),
    Id(Spanned<Id>),
    ProcCall(Spanned<ProcCall>),
}

#[derive(Debug, Clone)]
pub struct LValueIndex {
    pub of: Box<LValue>,
    pub index: Expr,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub cond: Expr,
    pub then_branch: Box<Stmt>,
    pub else_branch: Option<Box<Stmt>>,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub cond: Expr,
    pub body: Box<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub values: Vec<Expr>, 
}

#[derive(Debug, Clone)]
pub struct ProcCall {
    pub id: Spanned<Id>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Id(Spanned<Id>),
    Lit(Lit),
    Index(Spanned<ExprIndex>),
    Call(Spanned<ProcCall>),
    Length(Spanned<Box<Expr>>), //this expr includes extra tokens
    Unary(Spanned<ExprUOp>),
    Binary(Spanned<ExprBinOp>),
}

#[derive(Debug, Clone)]
pub struct ExprIndex {
    pub array: Box<Expr>,
    pub index: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct ExprUOp {
    pub op: Spanned<UOp>,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct ExprBinOp {
    pub op: Spanned<BinOp>,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
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

#[derive(Debug, Clone)]
pub enum Lit {
    IntLit(Spanned<IntLit>),
    BoolLit(Spanned<BoolLit>),
    CharLit(Spanned<CharLit>),
    ArrLit(ArrLit),
}

#[derive(Debug, Clone)]
pub enum ArrLit {
    StringLit(Spanned<String>),
    Array(Vec<Expr>),
}
