use logos::Logos;
use std::{fmt, num::ParseIntError};

use crate::{
    error,
    errors::{NoFileDiagnostic},
};

impl NoFileDiagnostic {
    fn from_lexer(lex: &mut logos::Lexer<'_, Token>) -> Self {
        let loc = lex.span();
        error!(loc, "unknown token").with_primary_label("this token")
    }
}

pub type Lexer<'input> = logos::Lexer<'input, Token>;

#[derive(Debug, Clone, PartialEq, Logos)]
#[logos(skip r"[ \t\n\f\r]+")]
#[logos(skip r"//[^\n]*")]
#[logos(error(NoFileDiagnostic, NoFileDiagnostic::from_lexer))]
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

    #[regex(r"'([^'\\]|\\(.|x\{[0-9A-Fa-f]{1,6}\}))'|''", parse_char)]
    CharLiteral(u32),

    #[regex(r#""([^"\\]|\\(.|x\{[0-9A-Fa-f]{1,6}\}))*""#, parse_str)]
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

fn parse_int(lex: &mut Lexer) -> Result<u64, NoFileDiagnostic> {
    lex.slice().parse::<u64>().map_err(|err: ParseIntError| {
        error!(lex.span(), "illegal integer literal: {}", err).with_primary_label(
            err.to_string().replace("number too extreme to fit in target type", "integer out of range"),
        )
    })
}

/// Parse a char literal of the form `'c'`, `'\\c'`, or `'\\x{HHHHHH}'`.
/// The surrounding quotes are stripped by this function.
fn parse_char(lex: &mut Lexer) -> Result<u32, NoFileDiagnostic> {
    let raw = lex.slice();
    if raw == "''" {
        return Err(error!(lex.span(), "empty character literal").with_primary_label("here"))
    };
    // Strip surrounding quotes.
    let inner = &raw[1..raw.len() - 1];
    decode_char_content(inner).ok_or_else(|| {
        error!(lex.span(), "invalid character literal: {}", raw)
            .with_primary_label( format!("cannot decode {}", raw))
    })
}

/// Parse a string literal of the form `"..."`. Individual character escapes
/// are validated; invalid ones produce a diagnostic.
fn parse_str(lex: &mut Lexer) -> Result<String, NoFileDiagnostic> {
    let raw = lex.slice();
    let inner = &raw[1..raw.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut it = inner.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        // Escape.
        let esc = it.next().ok_or_else(|| {
            error!(lex.span(), "unterminated escape in string literal")
                .with_primary_label("dangling backslash")
        })?;
        match esc {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            '\\' => out.push('\\'),
            '\'' => out.push('\''),
            '"' => out.push('"'),
            '0' => out.push('\0'),
            'x' => {
                // \x{HHHHHH}
                if it.next() != Some('{') {
                    return Err(error!(lex.span(), "malformed unicode escape")
                        .with_primary_label("expected `{` after `\\x`"));
                }
                let mut hex = String::new();
                loop {
                    match it.next() {
                        Some('}') => break,
                        Some(h) if h.is_ascii_hexdigit() => hex.push(h),
                        _ => {
                            return Err(error!(lex.span(), "malformed unicode escape")
                                .with_primary_label("expected hex digits and `}`"))
                        }
                    }
                }
                let codepoint = u32::from_str_radix(&hex, 16).map_err(|e| {
                    error!(lex.span(), "invalid unicode escape: {}", e)
                        .with_primary_label(e.to_string())
                })?;
                if let Some(ch) = char::from_u32(codepoint) {
                    out.push(ch);
                } else {
                    return Err(error!(lex.span(), "invalid unicode codepoint: U+{:X}", codepoint)
                        .with_primary_label("not a valid unicode scalar"));
                }
            }
            other => {
                return Err(error!(lex.span(), "unknown escape: \\{}", other)
                    .with_primary_label(format!("unknown escape: \\{}", other)))
            }
        }
    }
    Ok(out)
}

/// Decode the inside of a char literal (no surrounding quotes) into a
/// Unicode scalar value. Returns `None` on malformed input.
fn decode_char_content(inner: &str) -> Option<u32> {
    let mut chars = inner.chars();
    let first = chars.next()?;
    if first != '\\' {
        // Single character literal.
        if chars.next().is_some() {
            return None;
        }
        return Some(first as u32);
    }
    let esc = chars.next()?;
    match esc {
        'n' => Some('\n' as u32),
        't' => Some('\t' as u32),
        'r' => Some('\r' as u32),
        '\\' => Some('\\' as u32),
        '\'' => Some('\'' as u32),
        '"' => Some('"' as u32),
        '0' => Some(0),
        'x' => {
            if chars.next()? != '{' {
                return None;
            }
            let mut hex = String::new();
            loop {
                match chars.next()? {
                    '}' => break,
                    h if h.is_ascii_hexdigit() => hex.push(h),
                    _ => return None,
                }
            }
            if chars.next().is_some() {
                return None;
            }
            u32::from_str_radix(&hex, 16).ok()
        }
        _ => None,
    }
}

impl fmt::Display for Token {
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
            Token::BoolLiteral(b) => write!(f, "{}", b),
            Token::CharLiteral(c) => write!(f, "character {}", char::from_u32(*c).expect("illegal char somehow lexed").escape_default()),
            Token::StrLiteral(s) => write!(f, "string {}", s.escape_default()),
            Token::Identifier(s) => write!(f, "id {}", s),
            Token::Integer(n) => write!(f, "integer {}", n),
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
