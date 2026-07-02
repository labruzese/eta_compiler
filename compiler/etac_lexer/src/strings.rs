use crate::internal_error::InternalLexerError;

use super::{global_span, lexer_error, LogosLexer, Span};

const VALID_ESCAPES: &str = "valid escapes: '\\n', '\\t', '\\r', '\\\\', '\\'', '\\\"', '\\0', '\\x{..}'";

/// A cursor over the *inner* contents of a char/string literal (i.e. with the
/// surrounding quotes already stripped). `pos` is tracked as an absolute byte
/// offset into the *global* source so that spans produced here line up with
/// `global_span` / the rest of the diagnostics machinery, without needing the
/// caller to do any offset translation.
struct Cursor<'a> {
    input: &'a [u8],
    /// Absolute offset (into the global source) of `input[0]`.
    base: u32,
    /// Absolute offset (into the global source) of the next byte to read.
    pos: u32,
}

impl<'a> Cursor<'a> {
    /// `s` is the inner slice of the literal (quotes stripped). `base` is the
    /// absolute offset of `s`'s first byte in the global source.
    fn new(s: &'a str, base: u32) -> Self {
        Self {
            input: s.as_bytes(),
            base,
            pos: base,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get((self.pos - self.base) as usize).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let b = self.input.get((self.pos - self.base) as usize).copied();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    /// The `&str` spanning `[start, self.pos)`, where `start` is an absolute
    /// offset previously obtained from `self.loc()`.
    fn slice_from(&self, start: u32) -> &'a str {
        std::str::from_utf8(&self.input[(start - self.base) as usize..(self.pos - self.base) as usize]).unwrap()
    }

    /// Current absolute position (the offset of the next unread byte).
    fn loc(&self) -> u32 {
        self.pos
    }

    #[allow(clippy::cast_possible_truncation)]
    fn is_empty(&self) -> bool {
        self.pos - self.base >= self.input.len() as u32
    }
}

/// Advance the cursor past one full UTF-8 character, starting from a byte
/// that has *already been consumed* via `cursor.next()` (i.e. `start` is the
/// absolute offset of that already-consumed first byte). Consumes any
/// trailing UTF-8 continuation bytes (`0b10xxxxxx`) that follow, then returns
/// the decoded `char` together with the absolute end offset (one past the
/// last byte of the character) -- i.e. `cursor.loc()` after the walk.
///
/// This is used anywhere we need to report a diagnostic "about" a character
/// in the source and want the span to cover the *whole* character rather
/// than just its first byte -- which matters for any non-ASCII input, since
/// a byte-oriented `Cursor` would otherwise slice into the middle of a
/// multi-byte sequence.
///
/// Precondition: the byte at `start` has already been consumed (this just
/// walks any *remaining* continuation bytes); the caller is responsible for
/// having consumed at least the first byte before calling this.
fn finish_char(cursor: &mut Cursor, start: u32) -> (char, u32) {
    while cursor.peek().is_some_and(|b| b & 0b1100_0000 == 0b1000_0000) {
        cursor.next();
    }
    let s = cursor.slice_from(start);
    let c = s.chars().next().expect(
        "finish_char: slice_from(start) was empty -- caller must consume the first byte \
         of the character before calling finish_char",
    );
    (c, cursor.loc())
}

fn parse_hex(cursor: &mut Cursor, open: u32) -> Result<u32, InternalLexerError> {
    if cursor.next() != Some(b'{') {
        return Err(lexer_error! {
            span = Span::new(open, open + 2),
            message = "expected '{{' after '\\x'",
            plabel = "at this escape sequence"
        })
    }

    let mut value: u32 = 0;
    let mut digit_count: u32 = 0;
    let digits_start = cursor.loc();
    // Set once we've detected a problem (too many digits, or value out of
    // range) but want to keep consuming hex digits so the eventual error
    // span covers the *entire* run of digits the user typed, not just the
    // prefix up to the first offending digit.
    let mut pending_error: Option<(&'static str, &'static str, &'static str)> = None;

    loop {
        match cursor.peek() {
            Some(b'}') => {
                let close = cursor.loc();
                cursor.next();

                if digit_count == 0 {
                    // No digits between `{` and `}`.
                    return Err(lexer_error! {
                        span = Span::new(open, close),
                        message = "empty unicode escape expected non-empty hex between '{{' and '}}'",
                        plabel = "expected non-empty hex here",
                    });
                }

                if let Some(msg) = pending_error {
                    return Err(lexer_error! {
                        span = Span::new(digits_start, close),
                        message = msg.0,
                        plabel = msg.1,
                        note = msg.2,
                    });
                }

                return Ok(value);
            }
            Some(b @ (b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')) => {
                cursor.next();
                digit_count += 1;

                if digit_count > 6 {
                    // Already over the limit: keep scanning (without
                    // touching `value`, to avoid u32 overflow on
                    // pathologically long digit runs) until we find the
                    // closing `}` so the reported span covers everything
                    // the user wrote.
                    pending_error.get_or_insert(
                        ("too many hex digits in unicode escape",
                        "too many hex digits here",
                        "at most 6 hex digits are allowed between '{' and '}' (codepoints only go up to 10FFFF), e.g. '\\x{10FFFF}'"),
                    );
                    continue;
                }

                let digit = match b {
                    b'0'..=b'9' => b - b'0',
                    b'a'..=b'f' => b - b'a' + 10,
                    b'A'..=b'F' => b - b'A' + 10,
                    _ => unreachable!(),
                };
                value = (value << 4) | u32::from(digit);

                if value > 0x0010_FFFF {
                    // In range for *digit count* but the numeric value
                    // exceeds the maximum valid Unicode codepoint. Keep
                    // scanning (same reasoning as above) rather than
                    // bailing immediately, so the span covers the whole
                    // digit run instead of stopping at the first digit
                    // that tipped it over.
                    pending_error.get_or_insert((
                        "unicode escape out of range",
                        "this isn't a valid codepoint",
                        "the maximum valid codepoint is U+10FFFF ('\\x{10FFFF}')",
                    ));
                }
            }
            Some(_) => {
                // Not a hex digit and not '}': report the diagnostic over
                // the *entire* character at this position, not just its
                // first byte, so multi-byte UTF-8 input doesn't produce a
                // span that lands mid-character.
                let start = cursor.loc();
                cursor.next();
                let (ch, end) = finish_char(cursor, start);
                return Err(lexer_error! {
                    span = Span::new(start, end),
                    message = format!("invalid hex digit '{}' in unicode escape: '", ch),
                    plabel = "inside this unicode escape",
                    note = "only the digits 0-9 and letters a-f/A-F are allowed inside '\\x{...}",
                });
            }
            None => {
                // Ran off the end of the literal before finding a closing
                // '}'. Span the whole thing the user actually wrote, from
                // the opening '{' to wherever the literal's content ends
                // (i.e. as far as this Cursor can see), rather than just
                // pointing at the single byte where we gave up.
                let end = cursor.loc();
                return Err(lexer_error! {
                    span = Span::new(open, end),
                    message = "unterminated unicode escape",
                    plabel = "unicode escape unclosed here",
                    note = "expected a closing '}' before the end of the literal"
                });
            }
        }
    }
}

fn decode_escape(cursor: &mut Cursor) -> Result<char, InternalLexerError> {
    // `esc_start` is the position of the backslash itself; callers call
    // `cursor.next()` to consume the `\` *before* calling `decode_escape`,
    // so we reconstruct that position here for span purposes.
    let esc_start = cursor.loc() - 1;

    let b_pos = cursor.loc();
    let b = cursor.next().ok_or_else(|| {
        lexer_error! {
            span = Span::new(esc_start, esc_start),
            message = "dangling backslash",
            plabel = "closing quote is escaped here",
            note = "expected an escape character after '\\'",
        }
    })?;

    let esc = match b {
        b'n' => '\n',
        b't' => '\t',
        b'r' => '\r',
        b'\\' => '\\',
        b'\'' => '\'',
        b'"' => '"',
        b'0' => '\0',

        b'x' => {
            let esc_span = Span::new(esc_start, cursor.loc());
            let cp = parse_hex(cursor, esc_start)?;
            let end = cursor.loc();

            // Reject UTF-16 surrogate halves (U+D800–U+DFFF). These are
            // not valid Unicode scalar values and cannot be represented as
            // `char`. We catch this here — after parse_hex, before
            // constructing any `char` — so both char and string literals
            // get the error from a single place.
            if (0xD800..=0xDFFF).contains(&cp) {
                return Err(lexer_error! {
                    span = Span::new(esc_start, end),
                    message = "invalid unicode escape",
                    plabel = "this escape produces a surrogate half",
                    note = format!("U+{cp:04X} is a UTF-16 surrogate half and is not a valid \
                                    Unicode scalar value; surrogate halves (U+D800–U+DFFF) \
                                    cannot be used in escape sequences")
                });
            }

            // char::from_u32 accepts all non-surrogate codepoints ≤
            // U+10FFFF, which is exactly what parse_hex already allows
            // through, so this unwrap cannot fail.
            let _ = esc_span; // span was only needed for the surrogate error above
            char::from_u32(cp).expect(
                "decode_escape: parse_hex returned a non-surrogate codepoint that \
                 char::from_u32 rejected -- this is a bug",
            )
        }

        _ if b.is_ascii() => {
            let l = cursor.loc();
            return Err(lexer_error! {
                span = Span::new(esc_start, l),
                message = format!("unknown escape: '\\{}' is not a recognized escape sequence", b as char),
                plabel = "this isn't a recognized escape sequence",
                note = VALID_ESCAPES,
            });
        }

        _ => {
            // The byte after `\` is the first byte of a multi-byte UTF-8
            // character. Walk the rest of that character so both the
            // reported character and the error span are correct, instead
            // of misinterpreting a lone continuation byte via `as char`.
            let (ch, end) = finish_char(cursor, b_pos);
            return Err(lexer_error! {
                span = Span::new(esc_start, end),
                message = format!("unknown escape: '\\{ch}' is not a recognized escape sequence"),
                plabel = "this isn't a recognized escape sequence",
                note = VALID_ESCAPES,
            });
        }
    };

    Ok(esc)
}

pub fn parse_char<'a, 's>(lex: &'a mut LogosLexer<'s>) -> Result<u32, InternalLexerError> {
    let raw = lex.slice();
    let tok_span = global_span(lex);

    if raw == "''" {
        return Err(lexer_error!{
            span = tok_span,
            message = "empty character literal",
            plabel = "empty here",
            note = "a char literal must contain exactly one character",
        });
    }

    // Inner contents start 1 byte after the opening quote of the token.
    let inner_base = tok_span.lo + 1;
    let inner = &raw[1..raw.len() - 1];
    let mut cursor = Cursor::new(inner, inner_base);

    let value = match cursor.peek() {
        Some(b'\\') => {
            cursor.next();
            decode_escape(&mut cursor)?
        }
        Some(_) => {
            let start = cursor.loc();
            // Advance past one full UTF-8 character, not just one byte.
            cursor.next();
            let (c, _end) = finish_char(&mut cursor, start);
            c
        }
        None => {
            return Err(lexer_error! {
                span = tok_span,
                message = "invalid char literal",
                plabel = "this is not a valid char literal",
            });
        }
    };

    if !cursor.is_empty() {
        // Multiple characters (or a character plus trailing junk) inside
        // a single-quoted literal: span the *entire* literal, including
        // both quotes, so the user sees the whole offending token rather
        // than just the tail end of it.
        return Err(lexer_error! {
            span = tok_span,
            message = "too many characters in char literal",
            plabel = "this literal contains more than one character",
            note = " a char literal must contain exactly one character; did you mean to use a string literal (\"...\") instead?",
        });
    }

    Ok(value as u32)
}

pub fn parse_str(lex: &mut LogosLexer<'_>) -> Result<String, InternalLexerError> {
    let raw = lex.slice();
    let tok_span = global_span(lex);

    let inner_base = tok_span.lo + 1;
    let inner = &raw[1..raw.len() - 1];

    let mut cursor = Cursor::new(inner, inner_base);
    let mut out = String::with_capacity(inner.len());

    while let Some(b) = cursor.next() {
        match b {
            b'\\' => {
                let esc = decode_escape(&mut cursor)?;
                out.push(esc);
            }

            // ASCII fast path: push directly, no need to look at UTF-8
            // continuation bytes.
            b if b.is_ascii() => out.push(b as char),

            // Non-ASCII: walk back one byte (already consumed by `next()`)
            // and re-decode the full UTF-8 sequence as a `&str` so
            // multi-byte characters survive intact.
            _ => {
                let start = cursor.loc() - 1;
                let (c, _end) = finish_char(&mut cursor, start);
                out.push(c);
            }
        }
    }

    Ok(out)
}
