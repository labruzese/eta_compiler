//! Lexer
//!
//! Under the hood uses Logos but exports a compatibility layer more friendly to lalrpop.
//! Reports a Span within the global source cache.
#![allow(clippy::cast_possible_truncation)]

use std::{
    fmt::{self, Display},
    num::ParseIntError,
};

use etac_errors::{Diag, DiagCtxt, etac_error};
use etac_span::Span;
use logos::Logos;

mod internal_error;
use internal_error::{InternalLexerError, lexer_error};

pub trait ILexer<'dcx, 'src>: Iterator<Item = Result<(u32, Token<'src>, u32), Diag<'dcx>>> {}

fn global_span<'s>(lex: &logos::Lexer<'s, Token<'s>>) -> Span {
    Span::new(lex.extras + lex.span().start as u32, lex.extras + lex.span().end as u32)
}

fn lexer_error<'s>(lex: &mut logos::Lexer<'s, Token<'s>>) -> InternalLexerError {
    lexer_error! {
        span = global_span(lex),
        message = "unknown token",
        plabel = "this token",
    }
}

type LogosLexer<'s> = logos::Lexer<'s, Token<'s>>;

pub struct Lexer<'dcx, 'src> {
    diagc: &'dcx DiagCtxt,
    inner: logos::SpannedIter<'src, Token<'src>>,
}

impl<'dcx, 'src> Lexer<'dcx, 'src> {
    #[must_use]
    pub fn new(base: u32, source: &'src str, diag_context: &'dcx DiagCtxt) -> Self {
        Self {
            diagc: diag_context,
            inner: <Token as Logos>::lexer_with_extras(source, base).spanned(),
        }
    }
}

impl<'dcx, 'src> ILexer<'dcx, 'src> for Lexer<'dcx, 'src> {}
// transformed for lalrpop
impl<'dcx, 'src> Iterator for Lexer<'dcx, 'src> {
    type Item = Result<(u32, Token<'src>, u32), Diag<'dcx>>;

    fn next(&mut self) -> Option<Self::Item> {
        let (next, local_span) = self.inner.next()?;
        let base = self.inner.extras;
        let span = Span::new(base + local_span.start as u32, base + local_span.end as u32);
        match next {
            Ok(tok) => Some(Ok((span.lo, tok, span.hi))),
            Err(diag) => {
                let mut d = etac_error!(self.diagc, diag.span, "{}", diag.message);
                if let Some(l) = diag.plabel {
                    d = d.with_primary_label(l);
                }
                if let Some(l) = diag.note {
                    d = d.with_note(l);
                }
                Some(Err(d))
            }
        }
    }
}

mod strings;

#[derive(Debug, Clone, PartialEq, Logos)]
#[logos(lifetime = 's)]
#[logos(skip r"[ \t\n\f\r]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
#[logos(extras = u32)]
#[logos(error(InternalLexerError, lexer_error))]
pub enum Token<'s> {
    // Keywords
    #[token("use")]
    KeywordUse,
    #[token("length")]
    KeywordLength,
    #[token("while")]
    KeywordWhile,
    #[token("if")]
    KeywordIf,
    #[token("else")]
    KeywordElse,
    #[token("return")]
    KeywordReturn,
    #[token("int")]
    KeywordInt,
    #[token("bool")]
    KeywordBool,

    // Punctuation
    #[token(";")]
    SemiColon,
    #[token("_")]
    Discard,
    #[token(":")]
    OfType,
    #[token("=")]
    Assign,
    #[token(",")]
    Comma,

    #[token("true", |_| true)]
    #[token("false", |_| false)]
    BoolLiteral(bool),

    #[regex(r"'([^'\\]|\\(.|x\{[0-9A-Fa-f]{1,6}\}))*'", strings::parse_char)]
    CharLiteral(u32),

    #[regex(r#""([^"\\]|\\(.|x\{[0-9A-Fa-f]{1,6}\}))*""#, strings::parse_str)]
    StrLiteral(String),

    #[regex(r"[a-zA-Z][a-zA-Z0-9_']*", |lex| lex.slice())]
    Identifier(&'s str),

    #[regex(r"-?[1-9][0-9]*|0", parse_int)]
    Integer(i64),

    // Brackets and braces
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    BlockOpen,
    #[token("}")]
    BlockClose,

    // Arithmetic operators
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

    // Relational operators
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

    // Logical operators
    #[token("&")]
    Land,
    #[token("|")]
    Lor,
}

// Callbacks

fn parse_int(lex: &mut LogosLexer<'_>) -> Result<i64, InternalLexerError> {
    lex.slice().parse::<i64>().map_err(|err: ParseIntError| lexer_error! {
        span = global_span(lex),
        message = format!("illegal integer literal: {}", err),
        plabel = format!("this number is too {}", 
            if err.to_string().contains("large") { "large" } 
            else if err.to_string().contains("small") { "small" } 
            else { panic!("rust error message is weird") }),
        note = "eta only supports ints in the range [-2^63, 2^63)"
    })
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::KeywordUse => write!(f, "use"),
            Token::KeywordLength => write!(f, "length"),
            Token::KeywordWhile => write!(f, "while"),
            Token::KeywordIf => write!(f, "if"),
            Token::KeywordElse => write!(f, "else"),
            Token::KeywordReturn => write!(f, "return"),
            Token::KeywordInt => write!(f, "int"),
            Token::KeywordBool => write!(f, "bool"),
            Token::SemiColon => write!(f, ";"),
            Token::Discard => write!(f, "_"),
            Token::OfType => write!(f, ":"),
            Token::Assign => write!(f, "="),
            Token::Comma => write!(f, ","),
            Token::BoolLiteral(b) => write!(f, "{b}"),
            Token::CharLiteral(c) => write!(
                f,
                "character {}",
                char::from_u32(*c)
                    .expect("illegal char somehow lexed")
                    .escape_default()
                    .collect::<String>()
                    .replace("\\u{", "\\x{")
            ),
            Token::StrLiteral(s) => write!(
                f,
                "string {}",
                s.escape_default().collect::<String>().replace("\\u{", "\\x{")
            ),
            Token::Identifier(s) => write!(f, "id {s}"),
            Token::Integer(n) => write!(f, "integer {n}"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::BlockOpen => write!(f, "{{"),
            Token::BlockClose => write!(f, "}}"),
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
