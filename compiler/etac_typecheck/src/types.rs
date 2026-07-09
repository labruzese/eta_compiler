/// All EtaTypes
pub(crate) trait EtaType: std::fmt::Debug + std::any::Any {
    fn as_any(&self) -> &dyn std::any::Any where Self: std::marker::Sized { self }
}
impl EtaType for IntTy{}
impl EtaType for BoolTy{}
impl EtaType for ErrTy{}
impl EtaType for UnitTy{}
impl EtaType for VoidTy{}
impl EtaType for ArrayTy{}
impl EtaType for TupleTy{}
impl EtaType for FnTy{}


use etac_types_derive::EtaType;
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy)]
pub struct IntTy;

#[derive(Debug, Clone, Copy)]
pub struct BoolTy;

#[derive(Debug, Clone)]
pub struct ArrayTy {
    of: Box<VarTy>
}

#[derive(Debug, Clone, Copy)]
pub struct ErrTy;

#[derive(Debug, Clone, Copy)]
pub struct VoidTy;

#[derive(Debug, Clone, Copy)]
pub struct UnitTy;

pub type TupleTy = SmallVec::<[VarTy; 8]>;

#[derive(Debug, Clone)]
pub struct FnTy {
    pub from: TupleTy,
    pub to: TupleTy,
}

#[derive(Debug, Clone, EtaType)]
pub enum VarTy {
    Int(IntTy),
    Bool(BoolTy),
    Array(ArrayTy),
    Err(ErrTy),
}

#[derive(Debug, Clone, Copy, EtaType)]
pub enum StmtTy {
    Unit(UnitTy),
    Void(VoidTy),
}

#[derive(Debug, Clone, EtaType)]
pub enum AnyTy {
    Var(VarTy),
    Stmt(StmtTy),
    Fn(FnTy),
}

impl From<&etac_ast::TypeKind> for VarTy {
    fn from(value: &etac_ast::TypeKind) -> Self {
        match value {
            etac_ast::TypeKind::Array { of, .. } => VarTy::Array(ArrayTy { of: Box::new(VarTy::from(&of.kind)) }),
            etac_ast::TypeKind::Int => VarTy::Int(IntTy),
            etac_ast::TypeKind::Bool => VarTy::Bool(BoolTy),
        }
    }
}
