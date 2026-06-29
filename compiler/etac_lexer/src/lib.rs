//! Lexer
//!
//! Under the hood uses Logos but but exports a compatability layer more friendly to lalrpop.
//! Reports the a Span which is a span within the global source cache.
use std::{fmt::{self, Display}, num::ParseIntError};

use etac_errors::{error, Diagnostic};
use etac_span::{Span};
use logos::Logos;

fn global_span(lex: &logos::Lexer<'_, Token>) -> Span {
    Span::new(lex.extras + lex.span().start, lex.extras + lex.span().end)
}

fn lexer_error(lex: &mut logos::Lexer<'_, Token>) -> Diagnostic {
    error!(global_span(lex); "unknown token").with_primary_label("this token")
}

// api
type LogosLexer<'input> = logos::Lexer<'input, Token>;
pub struct Lexer<'input> {
    inner: logos::SpannedIter<'input, Token>,
}
impl<'source> Lexer<'source> {
    #[must_use]
    pub fn new(base: usize, source: &'source <Token as Logos>::Source) -> Self
    {
        Self { inner: <Token as Logos>::lexer_with_extras(source, base).spanned() }
    }
}

// transformed for lalrpop
impl Iterator for Lexer<'_> {
    type Item = Result<(usize, Token, usize), Diagnostic>;

    fn next(&mut self) -> Option<Self::Item> {
        let (next, local_span) = self.inner.next()?;
        let base = self.inner.extras;
        let span = Span::new(base + local_span.start, base + local_span.end);
        match next {
            Ok(tok) => Some(Ok((span.lo as usize, tok, span.hi as usize))),
            Err(diag) => Some(Err(diag)),
        }
    }
}

mod strings;

// logos
#[derive(Debug, Clone, PartialEq, Logos)]
#[logos(skip r"[ \t\n\f\r]+")]
#[logos(skip r"//[^\n]*")]
#[logos(extras = usize)]
#[logos(error(Diagnostic, lexer_error))]
pub enum Token {
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

    #[regex(r"[a-zA-Z][a-zA-Z0-9_']*", |lex| lex.slice().to_string())]
    Identifier(String),

    #[regex(r"[1-9][0-9]*|0", parse_int)]
    Integer(u64),

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

fn parse_int(lex: &mut LogosLexer) -> Result<u64, Diagnostic> {
    lex.slice().parse::<u64>().map_err(|err: ParseIntError| {
        error!(global_span(lex); "illegal integer literal: {}", err).with_primary_label(
            err.to_string().replace("number too extreme to fit in target type", "integer out of range"),
        )
    })
}

impl Display for Token {
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
            Token::CharLiteral(c) => write!(f, "character {}", char::from_u32(*c)
                                                                        .expect("illegal char somehow lexed")
                                                                        .escape_default()
                                                                        .collect::<String>()
                                                                        .replace("\\u{", "\\x{")),
            Token::StrLiteral(s) => write!(f, "string {}", s.escape_default().collect::<String>().replace("\\u{", "\\x{")),
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
