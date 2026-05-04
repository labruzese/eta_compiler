use crate::sources::span::EtaSpan;

/// Shorthand for wrapping an AST node in a `Spanned` with a file-aware span.
/// Usage: `sp!(file_id, start, end, node)`
#[macro_export]
macro_rules! sp {
    ($file:expr, $l:expr, $r:expr, $node:expr) => {
        Spanned::new(EtaSpan::new($file.clone(), $l, $r), $node)
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
pub enum Program {
    Prog {
        uses: Vec<Spanned<Use>>,
        definitions: Vec<Spanned<Definition>>,
    },
}

pub enum Interface {
    Interface(Vec<Spanned<MethodDecl>>)
}

#[derive(Debug, Clone)]
pub enum Use {
    Id(Id),
}

#[derive(Debug, Clone)]
pub enum Definition {
    Method(Method),
    GlobDecl(GlobDecl),
    Error,
}

#[derive(Debug, Clone)]
pub enum MethodDecl {
    MethodDecl {
        id: Id,
        params: Vec<Spanned<Decl>>,
        ret_types: Vec<Spanned<Type>>,
    },
}

#[derive(Debug, Clone)]
pub enum Method {
    Method {
        id: Id,
        params: Vec<Spanned<Decl>>,
        ret_types: Vec<Spanned<Type>>,
        body: Spanned<Block>,
    },
}

#[derive(Debug, Clone)]
pub enum GlobDecl {
    GlobDecl {
        id: Id,
        typ: Spanned<Type>,
        val: Option<Spanned<Value>>,
    },
}

#[derive(Debug, Clone)]
pub enum Value {
    IntLit(IntLit),
    BoolLit(BoolLit),
}

#[derive(Debug, Clone)]
pub enum Decl {
    Decl { id: Id, typ: Spanned<Type> },
}

#[derive(Debug, Clone)]
pub enum Type {
    SizedArray {
        of: Box<Spanned<Type>>,
        size: Spanned<Expr>,
    },
    UnsizedArray {
        of: Box<Spanned<Type>>,
    },
    Int,
    Bool,
}

#[derive(Debug, Clone)]
pub enum Block {
    Block { stmts: Vec<Spanned<Stmt>> },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assignment(Assignment),
    IfStmt(IfStmt),
    WhileStmt(WhileStmt),
    ReturnStmt(ReturnStmt),
    ProcCall(ProcCall),
    Block(Block),
    Decls { decls: Vec<Spanned<Decl>> },
    Error,
}

#[derive(Debug, Clone)]
pub enum Assignment {
    Assignment {
        targets: Vec<Spanned<AssignLeft>>,
        values: Vec<Spanned<Expr>>,
    },
}

#[derive(Debug, Clone)]
pub enum AssignLeft {
    LValue(LValue),
    Decl(Decl),
    Ignore,
}

#[derive(Debug, Clone)]
pub enum LValue {
    Index {
        of: Box<Spanned<LValue>>,
        index: Spanned<Expr>,
    },
    Id(Id),
    ProcCall(ProcCall),
}

#[derive(Debug, Clone)]
pub enum IfStmt {
    IfStmt {
        cond: Spanned<Expr>,
        then_branch: Box<Spanned<Stmt>>,
        else_branch: Option<Box<Spanned<Stmt>>>,
    },
}

#[derive(Debug, Clone)]
pub enum WhileStmt {
    WhileStmt {
        cond: Spanned<Expr>,
        body: Box<Spanned<Stmt>>,
    },
}

#[derive(Debug, Clone)]
pub enum ReturnStmt {
    ReturnStmt { values: Vec<Spanned<Expr>> },
}

#[derive(Debug, Clone)]
pub enum ProcCall {
    ProcCall { id: Id, args: Vec<Spanned<Expr>> },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Id(Id),
    Lit(Lit),
    Index {
        array: Box<Spanned<Expr>>,
        index: Box<Spanned<Expr>>,
    },
    Call(ProcCall),
    Length(Box<Spanned<Expr>>),
    Unary {
        op: UOp,
        expr: Box<Spanned<Expr>>,
    },
    Binary {
        op: BinOp,
        left: Box<Spanned<Expr>>,
        right: Box<Spanned<Expr>>,
    },
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
    IntLit(IntLit),
    BoolLit(BoolLit),
    CharLit(CharLit),
    ArrLit(ArrLit),
}

#[derive(Debug, Clone)]
pub enum ArrLit {
    StringLit(String),
    Array(Vec<Spanned<Expr>>),
}
