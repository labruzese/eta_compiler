mod collect;
mod context;
mod types;

macro_rules! already_declared {
    ($env:expr, $thing:literal, $name:expr, $first_declared:expr) => {
        etac_errors::etac_error! {
            $env.dcx, $env.span_table.get($first_declared), "{} {} already exists", $thing, $name;
            primary: "{} first defined here", $thing;
        }
    };
}

use etac_types_derive::EtaType;
use std::any::Any;

macro_rules! typecheck_wrapper {
    ($ast_node:tt.$field:ident: $inner_type:ty) => {
        paste::paste! {
            impl Typecheck for $ast_node {
                type Ty = <$inner_type as Typecheck>::Ty;
                fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
                    self.$field.typecheck(env)
                }
            }
        }
    };
}

macro_rules! typecheck_kind {
    ($ast_kind_node:ty { $($variant:ident),+ $(,)? }) => { paste::paste! {
        #[derive(Debug, Clone, EtaType)]
        pub enum [<$ast_kind_node Type>] {
            $($variant(<$variant as Typecheck>::Ty)),+
        }
        impl Typecheck for $ast_kind_node {
            type Ty = [<$ast_kind_node Type>];
            fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
                match self {
                    $($ast_kind_node::$variant(inner) => {
                        let ty = [<$ast_kind_node Type>]::$variant(inner.typecheck(env)?.clone());
                        let id = inner.node_id();
                        env.types.insert(id, Box::new(ty));
                        let any: &dyn Any = env.types.get(&id).unwrap().as_ref();
                        Ok(any.downcast_ref::<Self::Ty>().unwrap())
                    })+
                    $ast_kind_node::Error => Err(unsafe { ErrorGuaranteed::claim_already_emitted() })
                }
            }
        }
    }}
}

type Result<T> = std::result::Result<T, ErrorGuaranteed>;

/// Can be typechecked.
trait Typecheck {
    type Ty: types::EtaType;
    /// Updates enviornment by typechecking itself.
    /// Returns the deduced type of itself
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty>;
}

use etac_ast::*;
use etac_errors::ErrorGuaranteed;

// Program
// Interface
// Use
//
typecheck_wrapper!(Definition.kind: DefinitionKind);
typecheck_kind!(DefinitionKind { Method, GlobDecl });
typecheck_wrapper!(InterfaceItem.kind: InterfaceItemKind);
typecheck_kind!(InterfaceItemKind { MethodDecl });

impl Typecheck for MethodDecl {
    type Ty = types::FnTy;
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
        let ty = types::FnTy {
            from: self.params.iter().map(|decl| (&decl.typ.kind).into()).collect(),
            to: self.ret_types.iter().map(|typ| (&typ.kind).into()).collect(),
        };
        env.types.insert(self.node_id(), Box::new(ty.clone()));
        match env.scopes.declare_fn(self.node_id(), self.id.sym.clone(), ty) {
            Ok(context::FnEntry { ty, .. }) => Ok(ty),
            Err(context::FnEntry { declared, .. }) => {
                Err(already_declared!(env, "method", self.id.sym, *declared).emit())
            }
        }
    }
}

impl Typecheck for Method {
    type Ty = types::FnTy;
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
        let ty = types::FnTy {
            from: self.params.iter().map(|decl| (&decl.typ.kind).into()).collect(),
            to: self.ret_types.iter().map(|typ| (&typ.kind).into()).collect(),
        };
        env.types.insert(self.node_id(), Box::new(ty.clone()));
        match env.scopes.declare_fn(self.node_id(), self.id.sym.clone(), ty) {
            Ok(context::FnEntry { ty, .. }) => Ok(ty),
            Err(context::FnEntry { declared, .. }) => {
                Err(already_declared!(env, "method", self.id.sym, *declared).emit())
            }
        }
    }
}

impl Typecheck for GlobDecl {
    type Ty = types::UnitTy;
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
        let ty = self.typ.typecheck()?;
        env.types.insert(self.node_id(), Box::new(ty.clone()));
        match env.scopes.declare_var(self.node_id(), self.id.sym.clone(), ty) {
            Ok(_) => Ok(&types::UnitTy),
            Err(context::VarEntry { declared, .. }) => {
                Err(already_declared!(env, "variable", self.id.sym, *declared).emit())
            }
        }
    }
}

// Value
// ValueKind
// Decl
typecheck_wrapper!(Type.kind: TypeKind);
impl Typecheck for TypeKind {
    type Ty = types::VarTy;
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
        match self {
            TypeKind::Array { of, size } => types::VarTy::Array(of.typecheck(env).or(types::VarTy::Err(types::ErrTy))),
            TypeKind::Int => types::VarTy::Int(types::IntTy),
            TypeKind::Bool => types::VarTy::Bool(types::BoolTy),
            // Recovered at parse time (diagnostic already emitted); propagate
            // the error rather than re-reporting.
            TypeKind::Error => return Err(unsafe { ErrorGuaranteed::claim_already_emitted() }),
        }
    }
}
// Block
// Stmt
// StmtKind
// Target
// TargetKind
// LValue
// LValueKind
// ProcCall
// Expr
// ExprKind
// UOp
// BinOp
// Lit
// ArrLit
