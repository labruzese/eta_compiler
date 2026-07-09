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

macro_rules! delegate_typecheck {
    ($ast_node:tt.$field:ident { $($variant:ident),+ $(,)? }) => {
        paste::paste! {
            #[derive(Debug, Clone, EtaType)]
            pub enum [<$ast_node Type>] {
                $($variant(<$variant as Typecheck>::Ty)),+
            }
            impl Typecheck for $ast_node {
                type Ty = [<$ast_node Type>];
                fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
                    match &self.$field {
                        $([<$ast_node Kind>]::$variant(inner) => {
                            let ty = [<$ast_node Type>]::$variant(inner.typecheck(env)?.clone());
                            let id = inner.node_id();
                            Ok(env.types.assign_type(id, Box::new(ty)))
                        })+
                        [<$ast_node Kind>]::Error => Err(unsafe { ErrorGuaranteed::claim_already_emitted() })
                    }
                }
            }
        }
    };
}

type Result<T> = std::result::Result<T, ErrorGuaranteed>;

/// Can be typechecked.
trait Typecheck: etac_ast::AstNode {
    type Ty: types::EtaType;
    /// Updates enviornment by typechecking itself.
    /// Returns the deduced type of itself
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty>;
}

use etac_ast::*;
use etac_errors::ErrorGuaranteed;

// Program
impl Typecheck for Program {
    type Ty = types::UnitTy;
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
        self.uses.iter().for_each(|u| { u.typecheck(env); });
        self.definitions.iter().for_each(|d| { d.typecheck(env); });
        Ok(&types::UnitTy)
    }
}
// Interface
impl Typecheck for Use {
    type Ty = types::UnitTy;
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
        todo!()
    }
}

delegate_typecheck!(Definition.kind { Method, GlobDecl });
delegate_typecheck!(InterfaceItem.kind { MethodDecl });

impl Typecheck for MethodDecl {
    type Ty = types::FnTy;
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
        let ty = types::FnTy {
            from: self.params.iter().map(|decl| (&decl.typ.kind).into()).collect(),
            to: self.ret_types.iter().map(|typ| (&typ.kind).into()).collect(),
        };
        env.types.assign_type(self.node_id(), Box::new(ty.clone()));
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
        env.types.assign_type(self.node_id(), Box::new(ty.clone()));
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
        let ty = self.typ.typecheck(env)?;
        let ty_t = ty.clone();
        let ty_s = ty.clone();
        env.types.assign_type(self.node_id(), Box::new(ty_t));
        match env.scopes.declare_var(self.node_id(), self.id.sym.clone(), ty_s) {
            Ok(_) => Ok(&types::UnitTy),
            Err(context::VarEntry { declared, .. }) => {
                Err(already_declared!(env, "variable", self.id.sym, *declared).emit())
            }
        }
    }
}

#[derive(Debug,Clone,EtaType)]
pub enum ValueType {
    Int(types::IntTy),
    Bool(types::BoolTy)
}
impl Typecheck for Value {
    type Ty = ValueType;
    fn typecheck<'e>(&self,env: &'e mut context::Env) -> Result<&'e Self::Ty>{
        match &self.kind {
            ValueKind::Int(_value) => {
                // todo: validation on the int
                let ty = ValueType::Int(types::IntTy);
                Ok(env.types.assign_type(self.node_id(), Box::new(ty)))
            }
            ValueKind::Bool(_) => {
                let ty = ValueType::Bool(types::BoolTy);
                Ok(env.types.assign_type(self.node_id(), Box::new(ty)))
            }
        }
    }
}

// Decl
impl Typecheck for Type {
    type Ty = types::VarTy;
    fn typecheck<'e>(&self, env: &'e mut context::Env) -> Result<&'e Self::Ty> {
        let ty = match &self.kind {
            TypeKind::Array { of, .. } => {
                let inner = of.typecheck(env);
                let recovered = match inner {
                    Ok(inner_ty) => inner_ty.clone(),
                    Err(_errg) => types::VarTy::Err(types::ErrTy),
                };
                types::VarTy::Array(types::ArrayTy{ of: Box::new(recovered) })
            }
            TypeKind::Int => types::VarTy::Int(types::IntTy),
            TypeKind::Bool => types::VarTy::Bool(types::BoolTy),
            TypeKind::Error => return Err(unsafe { ErrorGuaranteed::claim_already_emitted() }),
        };
        Ok(env.types.assign_type(self.node_id(), Box::new(ty)))
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
