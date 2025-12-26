use logos::Logos;
use std::fmt;
use std::num::ParseIntError;

use crate::{
    error,
    errors::{self, Diagnostic},
};

impl<'a> Diagnostic<'a> {
    fn from_lexer(_lex: &mut logos::Lexer<'_, Token>) -> Self {
        error!("unknown token")
    }
}

pub type Lexer<'input> = logos::Lexer<'input, Token>;

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(
    skip r"[ \t\n\f]+", // whitespace
    skip r"//.*\n?", // // comments
    skip r"/\*([^*]|\*[^/])*\*/", // /* comments */ 
)]
#[logos(
    error(errors::Diagnostic<'s>, Diagnostic::from_lexer)
)]
pub enum Token {
    #[regex("[a-zA-Z][a-zA-Z0-9_’']*", |lex| lex.slice().to_string())]
    Identifier(String),
    #[regex("[1-9][0-9]*", |lex| lex.slice().parse())]
    Integer(i32),

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    #[token("*")]
    OperatorMul,
    #[token("*>>")]
    OperatorHighMul,
    #[token("/")]
    OperatorDiv,
    #[token("%")]
    OperatorMod,

    #[token("!")]
    OperatorNot,
    #[token("-")]
    Minus,
    #[token("+")]
    OperatorAdd,

    #[token("==")]
    RelOpEq,
    #[token("!=")]
    RelOpNeq,
    #[token(">")]
    RelOpGr,
    #[token(">=")]
    RelOpGe,
    #[token("<")]
    RelOpLt,
    #[token("<=")]
    RelOpLe,
    #[token("&")]
    Land,
    #[token("|")]
    Lor,
}

impl<'fid> From<ParseIntError> for Diagnostic<'fid> {
    fn from(err: ParseIntError) -> Self {
        error!("illegal integer literal: {}", err)
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Identifier(name) => write!(f, "id {}", name),
            Token::Integer(i) => write!(f, "integer {}", i),

            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),

            Token::OperatorMul => write!(f, "*"),
            Token::OperatorHighMul => write!(f, "*>>"),
            Token::OperatorDiv => write!(f, "/"),
            Token::OperatorMod => write!(f, "%"),

            Token::OperatorNot => write!(f, "!"),
            Token::Minus => write!(f, "-"),
            Token::OperatorAdd => write!(f, "+"),

            Token::RelOpEq => write!(f, "=="),
            Token::RelOpNeq => write!(f, "!="),
            Token::RelOpGr => write!(f, ">"),
            Token::RelOpGe => write!(f, ">="),
            Token::RelOpLt => write!(f, "<"),
            Token::RelOpLe => write!(f, "<="),
            Token::Land => write!(f, "&"),
            Token::Lor => write!(f, "|"),
        }
    }
}
