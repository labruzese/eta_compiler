use super::types;
use crate::context::{Env, FnEntry};
use etac_ast::*;
use etac_errors::etac_error;

// collects the global scope of a program
pub fn add_interface(env: &mut Env, interface: &Interface) {
    for item in &interface.items {

    }
}

pub fn collect_global(env: &mut Env, prog: &Program) {
    let definitions = &prog.definitions;
    for def in definitions {
        match &def.kind {
            DefinitionKind::Method(m) => {
                env.scopes.declare_fn(
                    m.node_id(),
                    m.id.sym.clone(),
                    types::FnTy {
                        from: m.params.iter().map(|decl| (&decl.typ.kind).into()).collect(),
                        to: m.ret_types.iter().map(|typ| (&typ.kind).into()).collect(),
                    },
                );
            }
            DefinitionKind::GlobDecl(gd) => { gd.typecheck(); }
            DefinitionKind::Error => (), // Error already recorded
        }
    }
}
