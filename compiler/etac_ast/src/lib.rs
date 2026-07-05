//! Abstract syntax tree for the Eta language.
//!
//!  * Carrier / Kind split: `Expr` { `node_id`, span, kind: `ExprKind` }, etc.
//!    The carrier struct owns identity (`node_id`) and location (span); the
//!    `*Kind` enum owns the shape. Carriers get ids; leaf kinds do not.
//!  * `Spanned<T>` wraps small things that need a location but don't earn a
//!    full node (operators, etc.) instead of a parallel `_span` field.
//!  * `Error` variants mark recovered regions. The parser only builds one after
//!    recording the recovery's diagnostic, but the variant itself carries no
//!    proof of that -- the drop-bomb on `Diag` is what guards against a lost
//!    diagnostic. (If a type-level proof pulls its weight anywhere, it is in
//!    typechecking, where `ErrorGuaranteed` may return.)
//!  * Plain `pub` fields, not accessors — the AST exists to be taken apart.
//!  * Node ids are handed out by a `NodeIdGen` threaded through the parser
//!    (deterministic, resettable), not a process-global atomic.

use etac_span::Span;

mod printer;

// ---- Core ids and spans ----

/// Stable identifier for a node. Assigned by `NodeIdGen`, not by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

impl NodeId {
    /// Placeholder for synthesized nodes before real ids are assigned.
    pub const DUMMY: NodeId = NodeId(u32::MAX);

    #[must_use]
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// Hands out fresh node ids. Thread one through the parser and call `fresh()`.
/// Reset between compilations / tests by constructing a new one.
#[derive(Debug, Default)]
pub struct NodeIdGen {
    next: u32,
}

impl NodeIdGen {
    #[must_use]
    pub fn new() -> Self {
        NodeIdGen { next: 0 }
    }

    pub fn fresh(&mut self) -> NodeId {
        let id = NodeId(self.next);
        self.next += 1;
        id
    }
}

/// Uniform span access for any node that carries one.
pub trait HasSpan {
    fn span(&self) -> Span;
}

/// Uniform id access for any carrier node.
pub trait HasNodeId {
    fn node_id(&self) -> NodeId;
}

/// A `T` paired with a source location, for leaves too small to be full nodes.
#[derive(Debug, Clone, Copy)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

pub fn respan<T>(span: Span, node: T) -> Spanned<T> {
    Spanned { node, span }
}

impl<T> HasSpan for Spanned<T> {
    fn span(&self) -> Span {
        self.span
    }
}

// ---- Node macros ----

/// A `*Kind`-style enum: shape only, no identity or location.
macro_rules! opaque {
    ($(#[$meta:meta])* $name:ident { $($body:tt)* }) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub enum $name {
            $($body)*
        }
    };
}

/// A carrier struct: owns `node_id` + `span`, plus public fields.
/// `new` takes the id explicitly (get it from `NodeIdGen::fresh`).
macro_rules! concrete {
    ($(#[$meta:meta])* $name:ident { $($field:ident : $type:ty),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub struct $name {
            pub node_id: NodeId,
            pub span: Span,
            $(pub $field: $type,)*
        }

        impl HasSpan for $name {
            fn span(&self) -> Span {
                self.span
            }
        }

        impl HasNodeId for $name {
            fn node_id(&self) -> NodeId {
                self.node_id
            }
        }

        impl $name {
            pub fn new(node_id: NodeId, span: Span, $($field: $type),*) -> Self {
                $name { node_id, span, $($field),* }
            }
        }
    };
}

// ---- Identifiers ----

concrete! {
    Ident {
        sym: String
    }
}

// ---- Top level ----

concrete! {
    Program {
        uses: Vec<Use>,
        definitions: Vec<Definition>
    }
}

concrete! {
    Interface {
        items: Vec<InterfaceItem>
    }
}

concrete! {
    Use {
        id: Ident
    }
}

concrete! {
    Definition {
        kind: DefinitionKind
    }
}

opaque! {
    DefinitionKind {
        Method(Method),
        GlobDecl(GlobDecl),
        Error,
    }
}

concrete! {
    InterfaceItem {
        kind: InterfaceItemKind
    }
}

opaque! {
    InterfaceItemKind {
        Decl(MethodDecl),
        Error,
    }
}

// ---- Methods & globals ----

concrete! {
    MethodDecl {
        id: Ident,
        params: Vec<Decl>,
        ret_types: Vec<Type>
    }
}

concrete! {
    Method {
        id: Ident,
        params: Vec<Decl>,
        ret_types: Vec<Type>,
        body: Block
    }
}

concrete! {
    GlobDecl {
        id: Ident,
        typ: Type,
        val: Option<Value>
    }
}

// NOTE: `Value` overlaps `Lit::{Int, Bool}`. It exists because a global
// initializer is a *constant*, not an arbitrary expression. If you'd rather not
// maintain two literal representations, drop `Value`/`ValueKind`, make this
// `val: Option<Lit>`, and enforce constness in the type checker.
concrete! {
    Value {
        kind: ValueKind
    }
}

opaque! {
    ValueKind {
        Int(i128),
        Bool(bool),
    }
}

concrete! {
    Decl {
        id: Ident,
        typ: Type
    }
}

// ---- Types ----

concrete! {
    Type {
        kind: TypeKind
    }
}

opaque! {
    TypeKind {
        Array { of: Box<Type>, size: Option<Box<Expr>> },
        Int,
        Bool,
    }
}

impl TypeKind {
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, TypeKind::Array { .. })
    }
}

// ---- Blocks & statements ----

concrete! {
    Block {
        stmts: Vec<Stmt>
    }
}

concrete! {
    Stmt {
        kind: StmtKind
    }
}

opaque! {
    StmtKind {
        Assign { targets: Vec<Target>, values: Vec<Expr> },
        If { cond: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>> },
        While { cond: Expr, body: Box<Stmt> },
        Return { values: Vec<Expr> },
        Call(ProcCall),
        Block(Block),
        Decls(Vec<Decl>),
        Error,
    }
}

// ---- Targets & lvalues ----

// `Target` has no node_id/span of its own; its payload carries one (except
// `Discard`). If you need to point a diagnostic at `_` specifically, promote
// this to a `concrete!` node or wrap it in `Spanned`.
opaque! {
    Target {
        LValue(LValue),
        Decl(Decl),
        Discard(Spanned<()>),
    }
}

concrete! {
    LValue {
        kind: LValueKind
    }
}

opaque! {
    LValueKind {
        Id(Ident),
        ProcCall(ProcCall),
        Index { array: Box<Expr>, index: Box<Expr> },
    }
}

// ---- Calls ----

concrete! {
    ProcCall {
        id: Ident,
        args: Vec<Expr>
    }
}

// ---- Expressions ----

concrete! {
    Expr {
        kind: ExprKind
    }
}

opaque! {
    ExprKind {
        Id(Ident),
        Lit(Lit),
        Index { array: Box<Expr>, index: Box<Expr> },
        Call(ProcCall),
        Length(Box<Expr>),
        Unary { op: Spanned<UOp>, operand: Box<Expr> },
        Binary { op: Spanned<BinOp>, lhs: Box<Expr>, rhs: Box<Expr> },
        Error,
    }
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
