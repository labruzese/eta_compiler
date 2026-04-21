use std::fmt::Debug;

mod printer;

pub type Id = String;
pub type IntLit = i128;
pub type BoolLit = bool;
pub type CharLit = char;

#[derive(Debug, Clone)]
pub enum Program {
    Prog {
        uses: Vec<Use>,
        definitions: Vec<Definition>,
    },
}

#[derive(Debug, Clone)]
pub enum Use {
    Id(Id),
}

#[derive(Debug, Clone)]
pub enum Definition {
    Method(Method),
    GlobDecl(GlobDecl),
}

#[derive(Debug, Clone)]
pub enum Method {
    Method {
        id: Id,
        params: Vec<Decl>,
        ret_types: Vec<Type>,
        body: Block,
    },
}

#[derive(Debug, Clone)]
pub enum GlobDecl {
    GlobDecl {
        id: Id,
        typ: Type,
        val: Option<Value>,
    },
}

#[derive(Debug, Clone)]
pub enum Value {
    IntLit(IntLit),
    BoolLit(BoolLit),
}

#[derive(Debug, Clone)]
pub enum Decl {
    Decl { id: Id, typ: Type },
}

#[derive(Debug, Clone)]
pub enum Type {
    SizedArray { of: Box<Type>, size: Expr },
    UnsizedArray { of: Box<Type> },
    Int,
    Bool,
}

#[derive(Debug, Clone)]
pub enum Block {
    Block { stmts: Vec<Stmt> },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assignment(Assignment),
    IfStmt(IfStmt),
    WhileStmt(WhileStmt),
    ReturnStmt(ReturnStmt),
    ProcCall(ProcCall),
    Block(Block),
    Decls { decls: Vec<Decl> },
}

#[derive(Debug, Clone)]
pub enum Assignment {
    Assignment {
        targets: Vec<AssignLeft>,
        values: Vec<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum AssignLeft {
    Var(Var),
    Decl(Decl),
    Ignore,
}

#[derive(Debug, Clone)]
pub enum Var {
    Index { of: Box<Var>, index: Expr },
    Id(Id),
}

#[derive(Debug, Clone)]
pub enum IfStmt {
    IfStmt {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
}

#[derive(Debug, Clone)]
pub enum WhileStmt {
    WhileStmt {
        cond: Expr,
        body: Box<Stmt>,
    },
}

#[derive(Debug, Clone)]
pub enum ReturnStmt {
    ReturnStmt { values: Vec<Expr> },
}

#[derive(Debug, Clone)]
pub enum ProcCall {
    ProcCall { id: Id, args: Vec<Expr> },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Id(Id),
    Lit(Lit),
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    Call(ProcCall),
    Length(Box<Expr>),
    Unary {
        op: UOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
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
    Array(Vec<Expr>),
}
