use std::{fmt, num::ParseIntError};

use etac_errors::{error, Diagnostic};
use etac_span::FileId;
use logos::Logos;

fn lexer_error(lex: &mut logos::Lexer<'_, Token>) -> Diagnostic {
    error!(&lex.extras, lex.span(); "unknown token").with_primary_label("this token")
}

// api
type LogosLexer<'input> = logos::Lexer<'input, Token>;
pub struct Lexer<'input> {
    inner: logos::SpannedIter<'input, Token>,
}
impl<'source> Lexer<'source> {
    pub fn new(file_id: FileId, source: &'source <Token as Logos>::Source) -> Self
    where
        <Token as Logos<'source>>::Extras: Default,
    {
        Self { inner: <Token as Logos>::lexer_with_extras(source, file_id).spanned() }
    }
}
// transformed for lalrpop
impl Iterator for Lexer<'_> {
    type Item = Result<(usize, Token, usize), Diagnostic>;

    fn next(&mut self) -> Option<Self::Item> {
        let (next, span) = self.inner.next()?;
        match next {
            Ok(tok) => Some(Ok((span.start, tok, span.end))),
            Err(diag) => Some(Err(diag)),
        }
    }
}

// logos
#[derive(Debug, Clone, PartialEq, Logos)]
#[logos(skip r"[ \t\n\f\r]+")]
#[logos(skip r"//[^\n]*")]
#[logos(extras = FileId)]
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

fn parse_int(lex: &mut LogosLexer) -> Result<u64, Diagnostic> {
    lex.slice().parse::<u64>().map_err(|err: ParseIntError| {
        error!(&lex.extras, lex.span(); "illegal integer literal: {}", err).with_primary_label(
            err.to_string().replace("number too extreme to fit in target type", "integer out of range"),
        )
    })
}

/// Parse a char literal of the form `'c'`, `'\\c'`, or `'\\x{HHHHHH}'`.
/// The surrounding quotes are stripped by this function.
fn parse_char(lex: &mut LogosLexer) -> Result<u32, Diagnostic> {
    let raw = lex.slice();
    if raw == "''" {
        return Err(error!(&lex.extras, lex.span(); "empty character literal")
            .with_primary_label("here"))
    };
    // Strip surrounding quotes.
    let inner = &raw[1..raw.len() - 1];
    decode_char_content(inner).ok_or_else(|| {
        error!(&lex.extras, lex.span(); "invalid character literal: {}", raw)
            .with_primary_label(format!("cannot decode {}", raw))
    })
}

/// Parse a string literal of the form `"..."`. Individual character escapes
/// are validated; invalid ones produce a diagnostic.
fn parse_str(lex: &mut LogosLexer) -> Result<String, Diagnostic> {
    let raw = lex.slice();
    let inner = &raw[1..raw.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let base = lex.span().start + 1; // +1 to skip the opening quote byte

    let mut it = inner.char_indices().peekable();

    while let Some((i, c)) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        // The backslash sits at base + i.
        let bs_span = base + i..base + i + 1;

        let (ei, esc) = it.next().ok_or_else(|| {
            error!(&lex.extras, bs_span.clone(); "unterminated escape in string literal")
                .with_primary_label("dangling backslash")
        })?;

        match esc {
            'n'  => out.push('\n'),
            't'  => out.push('\t'),
            'r'  => out.push('\r'),
            '\\' => out.push('\\'),
            '\'' => out.push('\''),
            '"'  => out.push('"'),
            '0'  => out.push('\0'),
            'x'  => {
                // expect \x{hhhhhh}
                match it.next() {
                    Some((_, '{')) => {}
                    _ => {
                        let s = base + ei..base + ei + 1;
                        return Err(error!(&lex.extras, s; "malformed unicode escape")
                            .with_primary_label("expected `{` after `\\x`"));
                    }
                }

                let mut hex = String::new();
                let mut hex_start = None;

                loop {
                    match it.next() {
                        Some((_, '}')) => break,
                        Some((j, h)) if h.is_ascii_hexdigit() => {
                            hex_start.get_or_insert(j);
                            hex.push(h);
                        }
                        Some((j, _)) => {
                            let s = base + j..base + j + 1;
                            return Err(
                                error!(&lex.extras, s; "malformed unicode escape")
                                    .with_primary_label("expected hex digits and `}`"),
                            );
                        }
                        None => {
                            // will always be a closing quote in this context
                            let Some((j, _)) = it.peek() else { 
                                return Err(
                                    error!(&lex.extras, base+i..lex.span().end; "unterminated unicode escape")
                                        .with_primary_label("expected hex digits and '}'")
                                );
                            };
                            let s = base + j..base + j + 1;
                            return Err(
                                error!(&lex.extras, s; "malformed unicode escape")
                                    .with_primary_label("expected to find `}`"),
                            );
                        }
                    }
                }

                if hex.is_empty() {
                    let s = base + i..base + ei + 3;
                    return Err(
                        error!(&lex.extras, s; "empty unicode escape")
                            .with_primary_label("expected at least one hex digit between `{` and `}`"),
                    );
                }

                let hex_span = {
                    let start = base + hex_start.unwrap_or(ei + 2);
                    start..start + hex.len()
                };

                let codepoint = u32::from_str_radix(&hex, 16).map_err(|e| {
                    error!(&lex.extras, hex_span.clone(); "invalid unicode escape: {}", e)
                        .with_primary_label(e.to_string())
                })?;

                match char::from_u32(codepoint) {
                    Some(ch) => out.push(ch),
                    None => {
                        return Err(
                            error!(&lex.extras, hex_span; "invalid unicode codepoint: U+{:X}", codepoint)
                                .with_primary_label("not a valid unicode scalar"),
                        )
                    }
                }
            }
            other => {
                let s = base + ei..base + ei + other.len_utf8();
                return Err(
                    error!(&lex.extras, s; "unknown escape: \\{}", other)
                        .with_primary_label(format!("unknown escape: \\{}", other)),
                );
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
            u32::from_str_radix(&hex, 16)
                .ok()
                .filter(|&cp| char::from_u32(cp).is_some())
        },
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
            Token::CharLiteral(c) => write!(f, "character {}", char::from_u32(*c)
                                                                        .expect("illegal char somehow lexed")
                                                                        .escape_default()
                                                                        .collect::<String>()
                                                                        .replace("\\u{", "\\x{")),
            Token::StrLiteral(s) => write!(f, "string {}", s.escape_default().collect::<String>().replace("\\u{", "\\x{")),
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
