use super::ty;
use crate::scope::Env;
use etac_ast::*;

// collects the global scope of a program
pub fn collect(prog: &Program) -> Env {
    let mut env = Env::new();
    let Program { uses: _, definitions } = prog;
    for def in definitions {
        let Definition { span: _, kind } = def;
        match kind {
            DefinitionKind::Method(Method {
                span: _,
                id,
                params,
                ret_types,
                body: _,
            }) => env.insert(
                id.sym.clone(),
                ty::TyCtx::Fn {
                    from: params.iter().map(|decl| (&decl.typ.kind).into()).collect(),
                    to: ret_types.iter().map(|typ| (&typ.kind).into()).collect(),
                },
            ),
            DefinitionKind::GlobDecl(GlobDecl {
                span: _,
                id,
                typ,
                val: _,
            }) => env.insert(id.sym.clone(), ty::TyCtx::Var((&typ.kind).into())),
            DefinitionKind::Error => (),
        }
    }
    env
}
