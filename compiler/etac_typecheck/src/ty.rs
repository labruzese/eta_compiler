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
