use logos::Logos;
use std::{fmt, num::ParseIntError, str::ParseBoolError};

use crate::{
    error,
    errors::{self, NoFileDiagnostic},
};

impl NoFileDiagnostic {
    fn from_lexer(lex: &mut logos::Lexer<'_, Token>) -> Self {
        let loc = lex.span();
        error!("unknown token").with_primary_label(&loc, "this token")
    }
}

pub type Lexer<'input> = logos::Lexer<'input, Token>;

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(
    skip r"[ \t\n\f]+", // whitespace
    skip r"//.*\n?", // // comments
    skip r"/\*([^*]|\*[^/])*\*/", // /* comments */ 
)]
#[logos(error(errors::NoFileDiagnostic, NoFileDiagnostic::from_lexer))]
pub enum Token {
    #[token("while")]
    KeywordWhile,
    #[token("if")]
    KeywordIf,
    #[token("else")]
    KeywordElse,
    #[token("return")]
    KeywordReturn,

    #[token("_")]
    Discard,
    #[token(":")]
    OfType,
    #[token("=")]
    Assign,
    #[token(",")]
    Comma,

    #[regex("true|false", |lex| lex.slice().parse()
        .map_err(|err: ParseBoolError| error!("illegal boolean literal: {}", err)
        .with_primary_label(&lex.span(), err.to_string().replace("target type", "boolean"))))]
    BoolLiteral(bool),
    #[regex(r#"'([^'\\]|\\.)*'"#, parse_char)]
    CharLiteral(u32),
    #[regex(r#""([^"\\]|\\.)*""#, unescape_string)]
    StrLiteral(String),
    #[regex("[a-zA-Z][a-zA-Z0-9_’']*", |lex| lex.slice().to_string())]
    Identifier(String),
    #[regex("-?[1-9][0-9]*|0", |lex| lex.slice().parse()
        .map_err(|err: ParseIntError| error!("illegal integer literal: {}", err)
        .with_primary_label(&lex.span(), err.to_string().replace("target type", "integer"))))]
    Integer(i32),

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

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::KeywordWhile => write!(f, "while"),
            Token::KeywordIf => write!(f, "if"),
            Token::KeywordElse => write!(f, "else"),
            Token::KeywordReturn => write!(f, "return"),

            Token::Discard => write!(f, "_"),
            Token::OfType => write!(f, ":"),
            Token::Assign => write!(f, "="),
            Token::Comma => write!(f, ","),

            Token::BoolLiteral(b) => write!(f, "{}", b),
            Token::StrLiteral(str) => write!(f, "string {}", escaped(str)),
            Token::CharLiteral(ch) => write!(
                f,
                "character {}",
                escaped(
                    &char::from_u32(*ch)
                        .expect("converting u32 back to char failed")
                        .to_string()
                )
            ),
            Token::Identifier(name) => write!(f, "id {}", name),
            Token::Integer(i) => write!(f, "integer {}", i),

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

/// Escape codes get lexed into their actual string during lexing.
/// Parsing does not need to worry about sanatizing string literals.
/// Supports: \n \r \t \0 \\ \" \' and \u{...hex...}
fn unescape_string<'s>(lex: &Lexer<'s>) -> Result<String, NoFileDiagnostic> {
    let span = lex.span();
    let s = lex.slice();
    let s = &s[1..s.len() - 1]; //shadow
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        // Handle escape
        let esc = chars.next().ok_or(
            error!("trailing backslash in escape").with_primary_label(&span, "in this string"),
        )?;
        match esc {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            '\'' => out.push('\''),
            'u' => {
                // Expect \u{...hex...}
                if chars.next() != Some('{') {
                    return Err(NoFileDiagnostic::error("expected {...} after \\u")
                        .with_primary_label(&span, "in this string"));
                }
                let mut hex = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        break;
                    }
                    hex.push(c);
                    chars.next();
                }
                if hex.is_empty() {
                    return Err(NoFileDiagnostic::error("empty unicode codepoint in \\u{}")
                        .with_primary_label(&span, "in this string"));
                }
                let code = u32::from_str_radix(hex.trim(), 16).map_err(|_| {
                    NoFileDiagnostic::error("invalid hex in \\u{...}")
                        .with_primary_label(&span, "in this string")
                })?;
                let ch = char::from_u32(code).ok_or(
                    NoFileDiagnostic::error("invalid Unicode scalar value")
                        .with_primary_label(&span, "in this string"),
                )?;
                out.push(ch);
            }
            other => {
                return Err(
                    error!("unknown escape: {}", other).with_primary_label(&span, "in this string")
                );
            }
        }
    }

    Ok(out)
}

fn parse_char<'s>(lex: &Lexer<'s>) -> Result<u32, NoFileDiagnostic> {
    let span = lex.span();
    let s = lex.slice();
    let s = &s[1..s.len() - 1]; //shadow
    match s {
        "\\n" => Ok('\n' as u32),
        "\\r" => Ok('\r' as u32),
        "\\t" => Ok('\t' as u32),
        "\\0" => Ok('\0' as u32),
        "\\\\" => Ok('\\' as u32),
        "\\\"" => Ok('\"' as u32),
        "\\\'" => Ok('\'' as u32),
        "\\u" => {
            let mut chars = s.chars().skip(2).peekable();
            // Expect \u{...hex...}
            if chars.next() != Some('{') {
                return Err(NoFileDiagnostic::error("expected {...} after \\u")
                    .with_primary_label(&span, "in this char"));
            }
            let mut hex = String::new();
            while let Some(&c) = chars.peek() {
                if c == '}' {
                    chars.next();
                    break;
                }
                hex.push(c);
                chars.next();
            }
            if hex.is_empty() {
                return Err(NoFileDiagnostic::error("empty unicode codepoint in \\u{}")
                    .with_primary_label(&span, "in this char"));
            }
            let code = u32::from_str_radix(hex.trim(), 16).map_err(|_| {
                NoFileDiagnostic::error("invalid hex in \\u{...}")
                    .with_primary_label(&span, "in this char")
            })?;
            let ch = char::from_u32(code).ok_or(
                NoFileDiagnostic::error("invalid Unicode scalar value")
                    .with_primary_label(&span, "in this char"),
            )?;
            Ok(ch as u32)
        }
        _ => Err(NoFileDiagnostic::error("invalid char").with_primary_label(&span, "this char")),
    }
}

fn escaped(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c.to_string()
            } else {
                c.escape_default().collect::<String>()
            }
        })
        .collect()
}
