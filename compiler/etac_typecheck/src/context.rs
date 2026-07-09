//! The typing context

use etac_ast::{NodeId, SpanTable};
use etac_errors::DiagCtxt;

use crate::types::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

#[derive(Debug)]
pub struct VarEntry {
    pub ty: VarTy,
    pub declared: NodeId,
}

#[derive(Debug)]
pub struct FnEntry {
    pub ty: FnTy,
    pub declared: NodeId,
}

#[derive(Debug)]
pub struct RetEntry {
    pub ty: TupleTy,
    pub declared: NodeId,
}

#[derive(Debug, Default)]
pub struct Scope {
    vars: HashMap<String, VarEntry>,
    fns: HashMap<String, FnEntry>,
    ret: Option<RetEntry>
}

#[derive(Debug, Default)]
struct Scopes(Vec<Scope>);

#[derive(Debug)]
pub struct Env<'dcx> {
    pub dcx: &'dcx DiagCtxt,
    pub span_table: &'dcx SpanTable,
    pub scopes: Scopes,
    pub types: HashMap<NodeId, Box<dyn EtaType>>
}

impl<'dcx> Env<'dcx> {
    pub fn new(dcx: &'dcx DiagCtxt, span_table: &'dcx SpanTable) -> Self {
        Self { dcx, span_table, scopes: Scopes(vec![Scope::default()]), types: HashMap::default() }
    }
}

impl Scopes {
    pub fn current_mut(&mut self) -> &mut Scope {
        self.0.last_mut().expect("at least 1 scope")
    }
    pub fn current(&self) -> &Scope {
        self.0.last().expect("at least 1 scope")
    }

    pub fn push(&mut self) {
        self.0.push(Scope::default());
    }
    pub fn pop(&mut self) {
        debug_assert!(self.0.len() > 1, "cannot pop the global scope");
        self.0.pop();
    }

    pub fn lookup_var(&self, bind: &str) -> Option<&VarEntry> {
        self.0.iter().rev().find_map(|s| s.vars.get(bind))
    }
    pub fn lookup_fn(&self, bind: &str) -> Option<&FnEntry> {
        self.0.iter().rev().find_map(|s| s.fns.get(bind))
    }
    pub fn lookup_ret(&self) -> Option<&RetEntry> {
        self.0.iter().rev().find_map(|s| s.ret.as_ref())
    }

    pub fn declare_var(
        &mut self,
        declaration: NodeId,
        binding: String,
        ty: VarTy,
    ) -> Result<&VarEntry, &VarEntry> {
        match self.current_mut().vars.entry(binding) {
            Entry::Occupied(entry) => Err(entry.into_mut()),
            Entry::Vacant(entry) => {
                Ok(entry.insert(VarEntry { ty, declared: declaration }))
            }
        }
    }

    pub fn declare_fn(
        &mut self,
        declaration: NodeId,
        binding: String,
        ty: FnTy,
    ) -> Result<&FnEntry, &FnEntry> {
        match self.current_mut().fns.entry(binding) {
            Entry::Occupied(entry) => Err(entry.into_mut()),
            Entry::Vacant(entry) => {
                Ok(entry.insert(FnEntry { ty, declared: declaration }))
            }
        }
    }

    pub fn declare_ret_type(&mut self, declaration: NodeId, ty: TupleTy) {
        self.current_mut().ret = Some(RetEntry { ty, declared: declaration });
    }
}


