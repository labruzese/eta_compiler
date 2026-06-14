// --- Types ---
pub enum Ty {
    Core(CoreTy),
    Tuple(TupleTy),
    Stmt(StmtTy),
}
pub type TupleTy = Vec<CoreTy>;
pub enum CoreTy {
    Int,
    Bool,
    Array(Box<CoreTy>),
    Err,
}
pub enum StmtTy {
    Unit,
    Void,
}

// --- Context ---
pub enum TyCtx {
    Var(CoreTy),
    Ret(TupleTy),
    Fn { from: TupleTy, to: TupleTy },
}

// --- Conversions ---
impl From<&etac_ast::TypeKind> for CoreTy {
    fn from(value: &etac_ast::TypeKind) -> Self {
        match value {
            etac_ast::TypeKind::UnsizedArray { of } |
            etac_ast::TypeKind::SizedArray { of, size: _ } => CoreTy::Array(Box::new(CoreTy::from(&of.kind))),
            etac_ast::TypeKind::Int => CoreTy::Int,
            etac_ast::TypeKind::Bool => CoreTy::Bool,
        }
    }
}
