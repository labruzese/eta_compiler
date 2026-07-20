use super::*;

pub trait Ast {
    fn recurse<V: Visitor>(&self, visitor: &mut V);
    fn visit<V: Visitor>(&self, visitor: &mut V);
}

pub trait Visitor {
    fn visit_error(&mut self, error: &Error);
    fn visit_program(&mut self, program: &Program);
    fn visit_interface(&mut self, interface: &Interface);
    fn visit_use(&mut self, r#use: &Use);
    fn visit_method_decl(&mut self, method_decl: &MethodDecl);
    fn visit_method(&mut self, method: &Method);
    fn visit_glob_decl(&mut self, glob_decl: &GlobDecl);
    fn visit_assign_stmt(&mut self, assign_stmt: &AssignStmt);
    fn visit_if_stmt(&mut self, if_stmt: &IfStmt);
    fn visit_while_stmt(&mut self, while_stmt: &WhileStmt);
    fn visit_return_stmt(&mut self, return_stmt: &ReturnStmt);
    fn visit_proc_call(&mut self, proc_call: &ProcCall);
    fn visit_block(&mut self, block: &Block);
    fn visit_multi_decl_stmt(&mut self, multi_decl_stmt: &MultiDeclStmt);
    fn visit_decl(&mut self, decl: &Decl);
    fn visit_int_type(&mut self, int_type: &IntType);
    fn visit_bool_type(&mut self, bool_type: &BoolType);
    fn visit_arr_type(&mut self, arr_type: &ArrType);
    fn visit_discard(&mut self, discard: &Discard);
    fn visit_ident(&mut self, ident: &Ident);
    fn visit_int_lit(&mut self, int_lit: &IntLit);
    fn visit_bool_lit(&mut self, bool_lit: &BoolLit);
    fn visit_char_lit(&mut self, char_lit: &CharLit);
    fn visit_arr_lit(&mut self, arr_lit: &ArrLit);
    fn visit_expr_index(&mut self, expr_index: &ExprIndex);
    fn visit_length_call(&mut self, length_call: &LengthCall);
    fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr);
    fn visit_neg_op(&mut self, neg_op: &NegOp);
    fn visit_not_op(&mut self, not_op: &NotOp);
    fn visit_binary_expr(&mut self, binary_expr: &BinaryExpr);
    fn visit_add_op(&mut self, add_op: &AddOp);
    fn visit_sub_op(&mut self, sub_op: &SubOp);
    fn visit_mul_op(&mut self, mul_op: &MulOp);
    fn visit_high_mul_op(&mut self, high_mul_op: &HighMulOp);
    fn visit_div_op(&mut self, div_op: &DivOp);
    fn visit_mod_op(&mut self, mod_op: &ModOp);
    fn visit_eq_op(&mut self, eq_op: &EqOp);
    fn visit_neq_op(&mut self, neq_op: &NeqOp);
    fn visit_lt_op(&mut self, lt_op: &LtOp);
    fn visit_gt_op(&mut self, gt_op: &GtOp);
    fn visit_le_op(&mut self, le_op: &LeOp);
    fn visit_ge_op(&mut self, ge_op: &GeOp);
    fn visit_and_op(&mut self, and_op: &AndOp);
    fn visit_or_op(&mut self, or_op: &OrOp);
}

/// This is a no-op that allows our macro to recur on any type where if it's not an AST it does
/// nothing
pub(crate) trait StopAstRecursion {
    /// This is a no-op that allows our macro to recur on any type where if it's not an AST it does
    /// nothing
    fn recurse<V: Visitor>(&self, visitor: &mut V);
    fn visit<V: Visitor>(&self, visitor: &mut V);
}
impl<T> StopAstRecursion for &T {
    fn recurse<V: Visitor>(&self, _visitor: &mut V) {}
    fn visit<V: Visitor>(&self, _visitor: &mut V) {}
}
