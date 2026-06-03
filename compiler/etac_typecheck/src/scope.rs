use crate::ty;
use std::collections::HashMap;

type Scope = HashMap<String, ty::TyCtx>;

pub struct Env {
    outer: Option<Box<Env>>,
    scope: Scope,
}

impl Env {
    pub fn new() -> Self {
        Self {
            outer: None,
            scope: Scope::new(),
        }
    }
    pub fn new_scope(self) -> Self {
        Self {
            outer: Some(Box::new(self)),
            scope: Scope::new(),
        }
    }
    pub fn lookup(&self, ident: &str) -> Option<&ty::TyCtx> {
        self.scope
            .get(ident)
            .or_else(|| self.outer.as_ref().and_then(|oenv| oenv.lookup(ident)))
    }
    pub fn insert(&mut self, ident: String, context: ty::TyCtx) {
        self.scope.insert(ident, context);
    }
    pub fn global_env<'s>(&'s self) -> &'s Self {
        self.outer.as_ref().map(|o| o.global_env()).unwrap_or(self)
    }
}
