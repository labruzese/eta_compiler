//! Abstract syntax tree for the Eta language.
//!
//!  * Every carrier struct owns a [`NodeId`], and spans live outside the tree in the session [`SpanTable`]. Recover a location with
//!    `spans.get(node.node_id)` or `spans.span_of(&node)`.
//!
//!  * Carrier / Kind split: `Expr` { `node_id`, kind: `ExprKind` }, etc. The
//!    carrier struct owns identity; the `*Kind` enum owns the shape. 
//!    Kinds are id-free  implements [`AstNode`] by destructuring to its 
//!    concrete payload's id.
//!
//!  * [`Leaf<T>`] pairs things too small to earn a full node (operators, `_`
//!    discards) with an id so their spans are still recorded precisely,
//!    instead of a parallel `_span` field.
//!
//!  * `Error` variants mark recovered regions. The parser only builds one
//!    after recording the recovery's diagnostic.
//!
//!  * One [`SpanTable`] is shared per compilation session and threaded through
//!    the parses of the program and every interface, so ids are unique across
//!    all trees. 
//!
//!    Later phases (typechecking over a desugared HIR) can key
//!    per-node facts by `NodeId` against the same flat table, and synthesized
//!    nodes can allocate ids of their own.

mod printer;

mod span_table;
pub use span_table::*;

// ---- Node macro ----

/// A carrier struct: owns identity (`node_id`, keying into the session
/// [`SpanTable`]) plus its public fields. `new` takes the id explicitly.
macro_rules! node {
    ($(#[$meta:meta])* $name:ident { $($field:ident : $type:ty),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub struct $name {
            pub node_id: NodeId,
            $(pub $field: $type,)*
        }
        impl AstNode for $name {
            fn node_id(&self) -> NodeId {
                self.node_id
            }
        }
        impl $name {
            pub fn new(node_id: NodeId, $($field: $type),*) -> Self {
                $name { node_id, $($field),* }
            }
        }
    };
}

/// A `T` too small to earn a carrier struct (operators, `_` discards), paired
/// with the id under which its span is recorded.
#[derive(Debug, Clone, Copy)]
pub struct Leaf<T> {
    pub node_id: NodeId,
    pub node: T,
}

impl<T> Leaf<T> {
    pub fn new(node_id: NodeId, node: T) -> Self {
        Leaf { node_id, node }
    }
}

impl<T> AstNode for Leaf<T> {
    fn node_id(&self) -> NodeId {
        self.node_id
    }
}

// ---- Identifiers ----

node! {
    Ident {
        sym: String
    }
}

// ---- Top level ----

node! {
    Program {
        uses: Vec<Use>,
        definitions: Vec<Definition>
    }
}

node! {
    Interface {
        items: Vec<InterfaceItem>
    }
}

node! {
    Use {
        id: Ident
    }
}

node! {
    Definition {
        kind: DefinitionKind
    }
}

#[derive(Debug, Clone)]
pub enum DefinitionKind {
    Method(Method),
    GlobDecl(GlobDecl),
    Error,
}

node! {
    InterfaceItem {
        kind: InterfaceItemKind
    }
}

#[derive(Debug, Clone)]
pub enum InterfaceItemKind {
    Decl(MethodDecl),
    Error,
}

// ---- Methods & globals ----

node! {
    MethodDecl {
        id: Ident,
        params: Vec<Decl>,
        ret_types: Vec<Type>
    }
}

node! {
    Method {
        id: Ident,
        params: Vec<Decl>,
        ret_types: Vec<Type>,
        body: Block
    }
}

node! {
    GlobDecl {
        id: Ident,
        typ: Type,
        val: Option<Value>
    }
}

// `Value` overlaps `Lit::{Int, Bool}` but is kept separate because a global
// initializer is a *constant*, not an arbitrary expression.
node! {
    Value {
        kind: ValueKind
    }
}

#[derive(Debug, Clone)]
pub enum ValueKind {
    Int(i128),
    Bool(bool),
}

node! {
    Decl {
        id: Ident,
        typ: Type
    }
}

// ---- Types ----

node! {
    Type {
        kind: TypeKind
    }
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Array { of: Box<Type>, size: Option<Box<Expr>> },
    Int,
    Bool,
}

impl TypeKind {
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, TypeKind::Array { .. })
    }
}

// ---- Blocks & statements ----

node! {
    Block {
        stmts: Vec<Stmt>
    }
}

node! {
    Stmt {
        kind: StmtKind
    }
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

/// `Target` has no `node_id` of its own; every variant's payload carries one,
/// so its `AstNode` impl destructures to the concrete payload.
#[derive(Debug, Clone)]
pub enum Target {
    LValue(LValue),
    Decl(Decl),
    Discard(Leaf<()>),
}

impl AstNode for Target {
    fn node_id(&self) -> NodeId {
        match self {
            Target::LValue(lv) => lv.node_id,
            Target::Decl(d) => d.node_id,
            Target::Discard(leaf) => leaf.node_id,
        }
    }
}

node! {
    LValue {
        kind: LValueKind
    }
}

#[derive(Debug, Clone)]
pub enum LValueKind {
    Id(Ident),
    ProcCall(ProcCall),
    Index { array: Box<Expr>, index: Box<Expr> },
}

// ---- Calls ----

node! {
    ProcCall {
        id: Ident,
        args: Vec<Expr>
    }
}

// ---- Expressions ----

node! {
    Expr {
        kind: ExprKind
    }
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Id(Ident),
    Lit(Lit),
    Index { array: Box<Expr>, index: Box<Expr> },
    Call(ProcCall),
    Length(Box<Expr>),
    Unary { op: Leaf<UOp>, operand: Box<Expr> },
    Binary { op: Leaf<BinOp>, lhs: Box<Expr>, rhs: Box<Expr> },
    Error,
}

// ---- Operators ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UOp {
    Neg,
    Not,
}

impl UOp {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            UOp::Neg => "-",
            UOp::Not => "!",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl BinOp {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        use BinOp::*;
        match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            HighMul => "*>>",
            Div => "/",
            Mod => "%",
            Eq => "==",
            Neq => "!=",
            Lt => "<",
            Gt => ">",
            Le => "<=",
            Ge => ">=",
            And => "&",
            Or => "|",
        }
    }

    /// Higher binds tighter.
    #[must_use]
    pub fn precedence(self) -> u8 {
        use BinOp::*;
        match self {
            Or => 1,
            And => 2,
            Eq | Neq => 3,
            Lt | Le | Gt | Ge => 4,
            Add | Sub => 5,
            Mul | Div | Mod | HighMul => 6,
        }
    }

    #[must_use]
    pub fn is_comparison(self) -> bool {
        use BinOp::*;
        matches!(self, Eq | Neq | Lt | Gt | Le | Ge)
    }

    /// `&` and `|` short-circuit
    #[must_use]
    pub fn is_short_circuit(self) -> bool {
        matches!(self, BinOp::And | BinOp::Or)
    }
}

// ---- Literals (id-free; locate via the enclosing Expr) ----

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
