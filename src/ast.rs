#[macro_export]
macro_rules! binop {
    ($l:expr, $op:ident, $r:expr) => {{ ast::Expr::Binary(Box::new($l), ast::BinOp::$op, Box::new($r)) }};
}

#[macro_export]
macro_rules! unaryop {
    ($op:ident, $r:expr) => {
        ast::Expr::Unary(ast::UnaryOp::$op, Box::new($r))
    };
}

#[derive(Debug)]
pub enum Expr {
    IntLiteral(i32),
    FnCall(String, Vec<Expr>),
    ArrayVar(String, Vec<i32>),
    Var(String),
    Unary(UnaryOp, Box<Expr>),
    Binary(Box<Expr>, BinOp, Box<Expr>),
}

#[derive(Debug)]
pub enum BinOp {
    Mul,
    HighMul,
    Div,
    Mod,
    Add,
    Sub,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Neq,
    Land,
    Lor,
}
#[derive(Debug)]
pub enum UnaryOp {
    Neg,
    Not,
}
