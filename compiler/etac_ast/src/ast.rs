pub mod interface;

use etac_ast_derive::{Ast, node, kind};
use interface::StopAstRecursion;
use interface::Visitor;
use crate::id::{AstNode, NodeId};

// Error

#[derive(Copy)] #[node] pub struct Error {
    node_id: NodeId<Self>
}

// Top level

#[node] pub struct Program {
    node_id: NodeId<Self>,
    uses: Vec<Use>,
    definitions: Vec<DefinitionKind>
}

#[node] pub struct Interface {
    node_id: NodeId<Self>,
    items: Vec<InterfaceItemKind>
}

#[node] pub struct Use {
    node_id: NodeId<Self>,
    id: Ident,
    interface: NodeId<Interface>,
}

// Definitions

#[kind] pub enum DefinitionKind {
    Method(Method),
    GlobDecl(GlobDecl),
    Error(Error),
}

#[kind] pub enum InterfaceItemKind {
    MethodDecl(MethodDecl),
    Error(Error),
}

#[node] pub struct MethodDecl {
    node_id: NodeId<Self>,
    id: Ident,
    params: Vec<Decl>,
    ret_types: Vec<TypeKind>
}

#[node] pub struct Method {
    node_id: NodeId<Self>,
    id: Ident,
    params: Vec<Decl>,
    ret_types: Vec<TypeKind>,
    body: Block
}

#[node] pub struct GlobDecl {
    node_id: NodeId<Self>,
    id: Ident,
    typ: TypeKind,
    val: Option<GlobValueKind>
}

#[kind] pub enum GlobValueKind {
    Int(IntLit),
    Bool(BoolLit),
}

// Statements

#[kind] pub enum StmtKind {
    Assign(AssignStmt),
    If(IfStmt),
    While(WhileStmt),
    Return(ReturnStmt),
    Call(ProcCall),
    Block(Block),
    Decls(MultiDeclStmt),
    Error(Error),
}

#[node] pub struct AssignStmt {
    node_id: NodeId<Self>,
    targets: Vec<TargetKind>,
    values: Vec<ExprKind>,
}

#[node] pub struct IfStmt {
    node_id: NodeId<Self>,
    cond: ExprKind,
    then_branch: NodeId<StmtKind>,
    else_branch: Option<NodeId<StmtKind>>
}

#[node] pub struct WhileStmt {
    node_id: NodeId<Self>,
    cond: ExprKind,
    body: NodeId<StmtKind>,
}

#[node] pub struct ReturnStmt {
    node_id: NodeId<Self>,
    values: Vec<ExprKind>,
}

#[node] pub struct ProcCall {
    node_id: NodeId<Self>,
    id: Ident,
    args: Vec<ExprKind>
}

#[node] pub struct Block {
    node_id: NodeId<Self>,
    stmts: Vec<StmtKind>
}

#[node] pub struct MultiDeclStmt {
    node_id: NodeId<Self>,
    decls: Vec<Decl>
}

#[node] pub struct Decl {
    node_id: NodeId<Self>,
    id: Ident,
    typ: TypeKind
}

// Types

#[kind] pub enum TypeKind {
    Array(ArrType),
    Int(IntType),
    Bool(BoolType),
    Error(Error),
}

#[derive(Copy)] #[node] pub struct IntType{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct BoolType{ node_id: NodeId<Self> }

#[node] pub struct ArrType{ 
    node_id: NodeId<Self>,
    of: NodeId<TypeKind>,
    size: Option<NodeId<ExprKind>> 
}

// Targets & LValues

#[kind] pub enum TargetKind {
    LValue(LValueKind),
    Decl(Decl),
    Discard(Discard),
}

#[kind] pub enum LValueKind {
    Id(Ident),
    ProcCall(ProcCall),
    Index(ExprIndex),
}

#[derive(Copy)] #[node] pub struct Discard {
    node_id: NodeId<Self>,
}

// ---- Expressions ----

#[kind] pub enum ExprKind {
    Id(Ident),
    Lit(LitKind),
    Index(ExprIndex),
    Call(ProcCall),
    Length(LengthCall),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Error(Error),
}

#[node] pub struct Ident {
    node_id: NodeId<Self>,
    sym: String,
}

#[kind] pub enum LitKind {
    Int(IntLit),
    Bool(BoolLit),
    Char(CharLit),
    Arr(ArrLit),
}

#[derive(Copy)] #[node] pub struct IntLit{ node_id: NodeId<Self>, value: i64 }
#[derive(Copy)] #[node] pub struct BoolLit{ node_id: NodeId<Self>, value: bool }
#[derive(Copy)] #[node] pub struct CharLit{ node_id: NodeId<Self>, value: char }
#[node] pub struct ArrLit{ node_id: NodeId<Self>, value: Vec<ExprKind> }

#[derive(Copy)] #[node] pub struct ExprIndex {
    node_id: NodeId<Self>,
    array: NodeId<ExprKind>,
    index: NodeId<ExprKind>,
}

#[derive(Copy)] #[node] pub struct LengthCall {
    node_id: NodeId<Self>,
    param: NodeId<ExprKind>,
}

// Operators
#[derive(Copy)] #[node] pub struct UnaryExpr {
    node_id: NodeId<Self>,
    op: UOpKind,
    operand: NodeId<ExprKind>,
}

#[derive(Copy)] #[kind] pub enum UOpKind {
    Neg(NegOp),
    Not(NotOp),
}
#[derive(Copy)] #[node] pub struct NegOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct NotOp{ node_id: NodeId<Self> }

#[derive(Copy)] #[node] pub struct BinaryExpr {
    node_id: NodeId<Self>,
    op: BinOpKind,
    lhs: NodeId<ExprKind>,
    rhs: NodeId<ExprKind>,
}

#[derive(Copy)] #[kind] pub enum BinOpKind {
    Add(AddOp),
    Sub(SubOp),
    Mul(MulOp),
    HighMul(HighMulOp),
    Div(DivOp),
    Mod(ModOp),
    Eq(EqOp),
    Neq(NeqOp),
    Lt(LtOp),
    Gt(GtOp),
    Le(LeOp),
    Ge(GeOp),
    And(AndOp),
    Or(OrOp),
}

#[derive(Copy)] #[node] pub struct AddOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct SubOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct MulOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct HighMulOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct DivOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct ModOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct EqOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct NeqOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct LtOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct GtOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct LeOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct GeOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct AndOp{ node_id: NodeId<Self> }
#[derive(Copy)] #[node] pub struct OrOp{ node_id: NodeId<Self> }
