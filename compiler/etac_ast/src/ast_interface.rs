use crate::*;

pub trait Ast {
    fn recurse<V: Visitor>(&self, visitor: &mut V);
}

pub trait Visitor {
    fn visit_program(&mut self, program: &mut Program);
    fn visit_use(&mut self, r#use: &Use);
    fn visit_definition(&mut self, definition: &Definition);
    fn visit_definitionkind(&mut self, definition_kind: &DefinitionKind);
    fn visit_interface(&mut self, interface: &Interface);
    fn visit_interfaceitem(&mut self, interface_item: &InterfaceItem);
    fn visit_interfaceitemkind(&mut self, interface_item_kind: &InterfaceItemKind);
    fn visit_methoddecl(&mut self, method_decl: &MethodDecl);
    fn visit_method(&mut self, method: &Method);
    fn visit_globdecl(&mut self, glob_decl: &GlobDecl);
    fn visit_value(&mut self, value: &Value);
    fn visit_value_kind(&mut self, value_kind: &ValueKind);
    fn visit_decl(&mut self, decl: &Decl);
    fn visit_type(&mut self, typ: &Type);
    fn visit_typekind(&mut self, type_kind: &TypeKind);
    fn visit_block(&mut self, block: &Block);
    fn visit_stmt(&mut self, stmt: &Stmt);
    fn visit_stmtkind(&mut self, stmt_kind: &StmtKind);
    fn visit_target(&mut self, target: &Target);
    fn visit_lvalue(&mut self, lvalue: &LValue);
    fn visit_lvaluekind(&mut self, lvalue_kind: &LValueKind);
    fn visit_proccall(&mut self, proc_call: &ProcCall);
    fn visit_expr(&mut self, expr: &Expr);
    fn visit_exprkind(&mut self, expr_kind: &ExprKind);
    fn visit_uop(&mut self, uop: &Leaf<UOp>);
    fn visit_binop(&mut self, binop: &Leaf<BinOp>);
    fn visit_lit(&mut self, lit: &Lit);
    fn visit_arrlit(&mut self, arr_lit: &ArrLit);
    fn visit_ident(&mut self, ident: &Ident);
}

pub trait VisitStop<A> {
    fn __visit_dispatch<V: Visitor, F: FnOnce(&mut V, &A)>(&self, _visit_: F, _visitor: &mut V) {}
}
impl<A> VisitStop<&A> for &A {}

pub trait VisitContinue<A> {
    fn __visit_dispatch<V: Visitor, F: FnOnce(&mut V, &A)>(&self, visit_: F, visitor: &mut V);
}
impl<A: Ast> VisitContinue<A> for A {
    fn __visit_dispatch<V: Visitor, F: FnOnce(&mut V, &A)>(&self, visit_: F, visitor: &mut V) {
        visit_(visitor, self)
    }
}
