use super::{error, global_span, Diagnostic, LogosLexer, Span};

const VALID_ESCAPES: &str = "valid escapes: '\\n', '\\t', '\\r', '\\\\', '\\'', '\\\"', '\\0', '\\x{..}'";

/// A cursor over the *inner* contents of a char/string literal (i.e. with the
/// surrounding quotes already stripped). `pos` is tracked as an absolute byte
/// offset into the *global* source so that spans produced here line up with
/// `global_span` / the rest of the diagnostics machinery, without needing the
/// caller to do any offset translation.
struct Cursor<'a> {
    input: &'a [u8],
    /// Absolute offset (into the global source) of `input[0]`.
    base: usize,
    /// Absolute offset (into the global source) of the next byte to read.
    pos: usize,
}

#[derive(Debug)]
enum Escape {
    Simple(char),
}

#[derive(Debug)]
struct Spanned<E> {
    esc: E,
    span: Span,
}

impl<'a> Cursor<'a> {
    /// `s` is the inner slice of the literal (quotes stripped). `base` is the
    /// absolute offset of `s`'s first byte in the global source.
    fn new(s: &'a str, base: usize) -> Self {
        Self {
            input: s.as_bytes(),
            base,
            pos: base,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos - self.base).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let b = self.input.get(self.pos - self.base).copied();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    /// The `&str` spanning `[start, self.pos)`, where `start` is an absolute
    /// offset previously obtained from `self.loc()`.
    fn slice_from(&self, start: usize) -> &'a str {
        std::str::from_utf8(&self.input[start - self.base..self.pos - self.base]).unwrap()
    }

    /// Current absolute position (the offset of the next unread byte).
    fn loc(&self) -> usize {
        self.pos
    }

    fn is_empty(&self) -> bool {
        self.pos - self.base >= self.input.len()
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
fn finish_char(cursor: &mut Cursor, start: usize) -> (char, usize) {
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

fn parse_hex(cursor: &mut Cursor) -> Result<u32, Diagnostic> {
    let open = cursor.loc() - 2;
    if cursor.next() != Some(b'{') {
        return Err(
            error!(Span::new(open, open+2); "expected '{{' after '\\x'").with_primary_label("at this escape sequence")
        );
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
                    return Err(error!(Span::new(open, close); "empty unicode escape expected non-empty hex between '{{' and '}}'").with_primary_label("expected non-empty hex here"));
                }

                if let Some(msg) = pending_error {
                    return Err(error!(Span::new(digits_start, close); "{}", msg.0)
                        .with_primary_label(msg.1)
                        .with_note(msg.2));
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
                return Err(error!(
                    Span::new(start, end);
                    "invalid hex digit '{}' in unicode escape: '", ch
                )
                .with_primary_label("inside this unicode escape")
                .with_note(
                    "only the digits 0-9 and \
                     letters a-f/A-F are allowed inside '\\x{...}",
                ));
            }
            None => {
                // Ran off the end of the literal before finding a closing
                // '}'. Span the whole thing the user actually wrote, from
                // the opening '{' to wherever the literal's content ends
                // (i.e. as far as this Cursor can see), rather than just
                // pointing at the single byte where we gave up.
                let end = cursor.loc();
                return Err(error!(
                    Span::new(open, end);
                    "unterminated unicode escape"
                )
                .with_primary_label("unicode escape unclosed here")
                .with_note("expected a closing '}}' before the end of the literal"));
            }
        }
    }
}

fn decode_escape(cursor: &mut Cursor) -> Result<Spanned<Escape>, Diagnostic> {
    // `esc_start` is the position of the backslash itself; callers call
    // `cursor.next()` to consume the `\` *before* calling `decode_escape`,
    // so we reconstruct that position here for span purposes.
    let esc_start = cursor.loc() - 1;

    let b_pos = cursor.loc();
    let b = cursor.next().ok_or_else(|| {
        error!(Span::new(esc_start, esc_start); "dangling backslash")
            .with_primary_label("closing quote is escaped here")
            .with_note("expected an escape character after '\\'")
    })?;

    let esc = match b {
        b'n' => Escape::Simple('\n'),
        b't' => Escape::Simple('\t'),
        b'r' => Escape::Simple('\r'),
        b'\\' => Escape::Simple('\\'),
        b'\'' => Escape::Simple('\''),
        b'"' => Escape::Simple('"'),
        b'0' => Escape::Simple('\0'),

        b'x' => {
            let esc_span = Span::new(esc_start, cursor.loc());
            let cp = parse_hex(cursor)?;
            let end = cursor.loc();

            // Reject UTF-16 surrogate halves (U+D800–U+DFFF). These are
            // not valid Unicode scalar values and cannot be represented as
            // `char`. We catch this here — after parse_hex, before
            // constructing any `char` — so both char and string literals
            // get the error from a single place.
            if (0xD800..=0xDFFF).contains(&cp) {
                return Err(error!(
                    Span::new(esc_start, end);
                    "invalid unicode escape",
                )
                .with_primary_label("this escape produces a surrogate half")
                .with_note(format!("U+{cp:04X} is a UTF-16 surrogate half and \
                     is not a valid Unicode scalar value; surrogate halves (U+D800–U+DFFF) \
                     cannot be used in escape sequences")));
            }

            // char::from_u32 accepts all non-surrogate codepoints ≤
            // U+10FFFF, which is exactly what parse_hex already allows
            // through, so this unwrap cannot fail.
            let _ = esc_span; // span was only needed for the surrogate error above
            Escape::Simple(char::from_u32(cp).expect(
                "decode_escape: parse_hex returned a non-surrogate codepoint that \
                 char::from_u32 rejected -- this is a bug",
            ))
        }

        _ if b.is_ascii() => {
            let l = cursor.loc();
            return Err(error!(
                Span::new(esc_start, l);
                "unknown escape: '\\{}' is not a recognized escape sequence", b as char
            )
            .with_primary_label("this isn't a recognized escape sequence")
            .with_note(VALID_ESCAPES));
        }

        _ => {
            // The byte after `\` is the first byte of a multi-byte UTF-8
            // character. Walk the rest of that character so both the
            // reported character and the error span are correct, instead
            // of misinterpreting a lone continuation byte via `as char`.
            let (ch, end) = finish_char(cursor, b_pos);
            return Err(error!(
                Span::new(esc_start, end);
                "unknown escape: '\\{ch}' is not a recognized escape sequence"
            )
            .with_primary_label("this isn't a recognized escape sequence")
            .with_note(VALID_ESCAPES));
        }
    };

    Ok(Spanned {
        esc,
        span: Span::new(esc_start, cursor.loc()),
    })
}

pub fn parse_char(lex: &mut LogosLexer) -> Result<u32, Diagnostic> {
    let raw = lex.slice();
    let tok_span = global_span(lex);

    if raw == "''" {
        return Err(
            error!(tok_span; "empty character literal")
                .with_primary_label("empty here")
                .with_note("a char literal must contain exactly one character"),
        );
    }

    // Inner contents start 1 byte after the opening quote of the token.
    let inner_base = tok_span.lo as usize + 1;
    let inner = &raw[1..raw.len() - 1];
    let mut cursor = Cursor::new(inner, inner_base);

    let value = match cursor.peek() {
        Some(b'\\') => {
            cursor.next();
            decode_escape(&mut cursor)?.esc
        }
        Some(_) => {
            let start = cursor.loc();
            // Advance past one full UTF-8 character, not just one byte.
            cursor.next();
            let (c, _end) = finish_char(&mut cursor, start);
            Escape::Simple(c)
        }
        None => {
            return Err(error!(tok_span; "invalid char literal").with_primary_label("this is not a valid char literal"));
        }
    };

    if !cursor.is_empty() {
        // Multiple characters (or a character plus trailing junk) inside
        // a single-quoted literal: span the *entire* literal, including
        // both quotes, so the user sees the whole offending token rather
        // than just the tail end of it.
        return Err(error!(
            tok_span;
            "too many characters in char literal"
        )
        .with_primary_label("this literal contains more than one character")
        .with_note(" a char literal must contain exactly one character; did you mean to use a string literal (\"...\") instead?"));
    }

    Ok(match value {
        Escape::Simple(c) => c as u32,
    })
}

pub fn parse_str(lex: &mut LogosLexer) -> Result<String, Diagnostic> {
    let raw = lex.slice();
    let tok_span = global_span(lex);

    let inner_base = tok_span.lo as usize + 1;
    let inner = &raw[1..raw.len() - 1];

    let mut cursor = Cursor::new(inner, inner_base);
    let mut out = String::with_capacity(inner.len());

    while let Some(b) = cursor.next() {
        match b {
            b'\\' => {
                let Spanned { esc, .. } = decode_escape(&mut cursor)?;

                match esc {
                    Escape::Simple(c) => out.push(c),
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Token;
    use logos::Logos;

    // ---------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------

    /// Build a `Cursor` over `s` with `base = 0`, for tests that don't care
    /// about absolute offsets.
    fn cursor(s: &str) -> Cursor<'_> {
        Cursor::new(s, 0)
    }

    /// Build a `Cursor` over `s` with an arbitrary non-zero base, to make
    /// sure span math doesn't accidentally assume `base == 0`.
    fn cursor_at(s: &str, base: usize) -> Cursor<'_> {
        Cursor::new(s, base)
    }

    /// Lex a full source string (e.g. `r#""hi""#` or `"'a'"`) and return the
    /// single resulting token, panicking if there wasn't exactly one or if
    /// it didn't match successfully. `extras` (the base offset) is 0.
    fn lex_one(src: &'static str) -> Result<Token, Diagnostic> {
        let mut lexer = Token::lexer_with_extras(src, 0);
        let tok = lexer.next().expect("expected exactly one token, got none");
        assert!(
            lexer.next().is_none() || tok.is_err(),
            "expected exactly one token, got more than one"
        );
        tok
    }

    fn lex_char(src: &'static str) -> Result<u32, Diagnostic> {
        match lex_one(src)? {
            Token::CharLiteral(c) => Ok(c),
            other => panic!("expected CharLiteral, got {other:?}"),
        }
    }

    fn lex_str(src: &'static str) -> Result<String, Diagnostic> {
        match lex_one(src)? {
            Token::StrLiteral(s) => Ok(s),
            other => panic!("expected StrLiteral, got {other:?}"),
        }
    }

    /// Assert that a diagnostic's message contains `needle`, for tests that
    /// want to confirm error *content* rather than just `is_err()`.
    fn assert_msg_contains(diag: &Diagnostic, needle: &str) {
        let msg = format!("{diag:?}");
        assert!(
            msg.contains(needle),
            "expected diagnostic to contain {needle:?}, got: {msg}"
        );
    }

    // ---------------------------------------------------------------
    // Cursor
    // ---------------------------------------------------------------

    #[test]
    fn cursor_peek_does_not_advance() {
        let c = cursor("ab");
        assert_eq!(c.peek(), Some(b'a'));
        assert_eq!(c.peek(), Some(b'a'));
    }

    #[test]
    fn cursor_next_advances_and_returns_byte() {
        let mut c = cursor("ab");
        assert_eq!(c.next(), Some(b'a'));
        assert_eq!(c.next(), Some(b'b'));
        assert_eq!(c.next(), None);
    }

    #[test]
    fn cursor_next_past_eof_does_not_advance_pos() {
        let mut c = cursor("a");
        assert_eq!(c.next(), Some(b'a'));
        let pos_at_eof = c.loc();
        assert_eq!(c.next(), None);
        assert_eq!(c.next(), None);
        assert_eq!(c.loc(), pos_at_eof);
    }

    #[test]
    fn cursor_is_empty() {
        let mut c = cursor("a");
        assert!(!c.is_empty());
        c.next();
        assert!(c.is_empty());
    }

    #[test]
    fn cursor_is_empty_on_empty_input() {
        let c = cursor("");
        assert!(c.is_empty());
        assert_eq!(c.peek(), None);
    }

    #[test]
    fn cursor_loc_respects_base() {
        let c = cursor_at("xyz", 100);
        assert_eq!(c.loc(), 100);
    }

    #[test]
    fn cursor_slice_from_respects_base() {
        let mut c = cursor_at("hello", 50);
        let start = c.loc(); // 50
        c.next(); // consumes 'h'
        c.next(); // consumes 'e'
        assert_eq!(c.slice_from(start), "he");
    }

    #[test]
    fn cursor_slice_from_multibyte_utf8() {
        let s = "héllo";
        let mut c = cursor(s);
        let start = c.loc();
        c.next();
        c.next();
        c.next();
        assert_eq!(c.slice_from(start), "h\u{e9}");
    }

    // ---------------------------------------------------------------
    // finish_char
    // ---------------------------------------------------------------

    #[test]
    fn finish_char_ascii_single_byte() {
        let mut c = cursor("a");
        let start = c.loc();
        c.next(); // consume 'a'
        let (ch, end) = finish_char(&mut c, start);
        assert_eq!(ch, 'a');
        assert_eq!(end, c.loc());
    }

    #[test]
    fn finish_char_two_byte_utf8() {
        // 'é' is 2 bytes.
        let mut c = cursor("é");
        let start = c.loc();
        c.next(); // consume first byte of 'é'
        let (ch, end) = finish_char(&mut c, start);
        assert_eq!(ch, 'é');
        assert_eq!(end, start + 2);
        assert!(c.is_empty());
    }

    #[test]
    fn finish_char_three_byte_utf8() {
        // '€' is 3 bytes.
        let mut c = cursor("€");
        let start = c.loc();
        c.next();
        let (ch, end) = finish_char(&mut c, start);
        assert_eq!(ch, '€');
        assert_eq!(end, start + 3);
    }

    #[test]
    fn finish_char_four_byte_utf8() {
        // '🦀' is 4 bytes.
        let mut c = cursor("🦀");
        let start = c.loc();
        c.next();
        let (ch, end) = finish_char(&mut c, start);
        assert_eq!(ch, '🦀');
        assert_eq!(end, start + 4);
    }

    #[test]
    fn finish_char_stops_before_next_character() {
        // Make sure walking continuation bytes of the *first* char doesn't
        // accidentally swallow bytes belonging to a *second* char.
        let mut c = cursor("éb");
        let start = c.loc();
        c.next();
        let (ch, end) = finish_char(&mut c, start);
        assert_eq!(ch, 'é');
        assert_eq!(end, start + 2);
        assert_eq!(c.peek(), Some(b'b'));
    }

    #[test]
    fn finish_char_respects_nonzero_base() {
        let mut c = cursor_at("é", 1000);
        let start = c.loc();
        c.next();
        let (ch, end) = finish_char(&mut c, start);
        assert_eq!(ch, 'é');
        assert_eq!(start, 1000);
        assert_eq!(end, 1002);
    }

    // ---------------------------------------------------------------
    // parse_hex
    // ---------------------------------------------------------------

    fn hex(s: &str) -> Result<u32, Diagnostic> {
        // `s` should be the bytes *after* the leading `\x`, e.g. "{41}".
        let mut c = cursor(s);
        parse_hex(&mut c)
    }

    fn hex_at(s: &str, base: usize) -> Result<u32, Diagnostic> {
        let mut c = cursor_at(s, base);
        parse_hex(&mut c)
    }

    #[test]
    fn hex_basic_lowercase() {
        assert_eq!(hex("{41}").unwrap(), 0x41);
    }

    #[test]
    fn hex_basic_uppercase_digits() {
        assert_eq!(hex("{FF}").unwrap(), 0xFF);
    }

    #[test]
    fn hex_lowercase_letters_value_correct() {
        assert_eq!(hex("{a}").unwrap(), 0xa);
        assert_eq!(hex("{f}").unwrap(), 0xf);
        assert_eq!(hex("{ab}").unwrap(), 0xab);
        assert_eq!(hex("{ff}").unwrap(), 0xff);
    }

    #[test]
    fn hex_uppercase_letters_value_correct() {
        assert_eq!(hex("{A}").unwrap(), 0xA);
        assert_eq!(hex("{F}").unwrap(), 0xF);
        assert_eq!(hex("{AB}").unwrap(), 0xAB);
    }

    #[test]
    fn hex_mixed_case() {
        assert_eq!(hex("{aB}").unwrap(), 0xAB);
    }

    #[test]
    fn hex_full_codepoint() {
        assert_eq!(hex("{10FFFF}").unwrap(), 0x0010_FFFF);
    }

    #[test]
    fn hex_zero() {
        assert_eq!(hex("{0}").unwrap(), 0);
    }

    #[test]
    fn hex_missing_open_brace() {
        assert!(hex("41}").is_err());
    }

    #[test]
    fn hex_empty_braces_is_error() {
        let err = hex("{}");
        assert!(err.is_err());
    }

    #[test]
    fn hex_empty_braces_error_message() {
        let err = hex("{}").unwrap_err();
        assert_msg_contains(&err, "empty unicode escape");
    }

    #[test]
    fn hex_empty_braces_span_covers_braces() {
        // digits_start == close == position of '}' since there are no
        // digits at all; span should be the (zero-width) gap between '{'
        // and '}'.
        let mut c = cursor_at("{}", 10);
        let err = parse_hex(&mut c).unwrap_err();
        let span = err.loc.expect("expected a span on this diagnostic");
        assert_eq!(span.lo as usize, 11); // just after '{'
        assert_eq!(span.hi as usize, 11); // at '}'
    }

    #[test]
    fn hex_unterminated_is_error() {
        let err = hex("{41");
        assert!(err.is_err());
    }

    #[test]
    fn hex_unterminated_error_message() {
        let err = hex("{41").unwrap_err();
        assert_msg_contains(&err, "unterminated unicode escape");
    }

    #[test]
    fn hex_unterminated_span_covers_open_brace_to_end_of_input() {
        // open '{' at base+0, digits "41" at base+1..base+3, then EOF.
        // Span should run from the '{' to wherever the cursor's input
        // ends (i.e. the full extent of what was actually written),
        // not just a single point at the EOF position.
        let err = hex_at("{41", 5).unwrap_err();
        let span = err.loc.unwrap();
        assert_eq!(span.lo as usize, 5); // the '{'
        assert_eq!(span.hi as usize, 8); // end of "41" (3 chars consumed after base 5)
    }

    #[test]
    fn hex_unterminated_with_no_digits_at_all() {
        // Just "{" and then EOF immediately -- still must not panic, and
        // span should still run from the '{' to end of input (same point,
        // since there's nothing after it).
        let err = hex_at("{", 5).unwrap_err();
        let span = err.loc.unwrap();
        assert_eq!(span.lo as usize, 5);
        assert_eq!(span.hi as usize, 6);
        assert_msg_contains(&err, "unterminated unicode escape");
    }

    #[test]
    fn hex_invalid_digit_is_error() {
        assert!(hex("{4g}").is_err());
        assert!(hex("{!}").is_err());
    }

    #[test]
    fn hex_invalid_digit_error_message() {
        let err = hex("{4g}").unwrap_err();
        assert_msg_contains(&err, "invalid hex digit");
    }

    #[test]
    fn hex_invalid_digit_span_is_single_ascii_char() {
        // base 0: '{' at 0, '4' at 1, 'g' at 2.
        let err = hex_at("{4g}", 0).unwrap_err();
        let span = err.loc.unwrap();
        assert_eq!(span.lo as usize, 2);
        assert_eq!(span.hi as usize, 3);
    }

    #[test]
    fn hex_invalid_digit_multibyte_spans_full_character() {
        // 'é' (2 bytes) as the offending "digit": the span must cover
        // both bytes, not just the first one, and the error message
        // should mention the actual decoded character.
        let err = hex_at("{4é}", 0).unwrap_err();
        let span = err.loc.unwrap();
        assert_eq!(span.lo as usize, 2);
        assert_eq!(span.hi as usize, 4); // 'é' is 2 bytes: [2,4)
        assert_msg_contains(&err, "é");
    }

    #[test]
    fn hex_invalid_digit_four_byte_multibyte_spans_full_character() {
        // '🦀' is 4 bytes; make sure the walk doesn't stop short.
        let err = hex_at("{🦀}", 0).unwrap_err();
        let span = err.loc.unwrap();
        assert_eq!(span.lo as usize, 1);
        assert_eq!(span.hi as usize, 5); // 4-byte char starting at offset 1
    }

    #[test]
    fn hex_out_of_range_is_error() {
        assert!(hex("{FFFFFF}").is_err());
    }

    #[test]
    fn hex_out_of_range_error_message() {
        let err = hex("{FFFFFF}").unwrap_err();
        assert_msg_contains(&err, "out of range");
    }

    #[test]
    fn hex_out_of_range_span_covers_all_digits() {
        // "{FFFFFF}": 6 digits, value 0xFFFFFF > 0x10FFFF. Within the
        // 6-digit budget, so this is purely a range error, not a
        // digit-count error. Span should cover all 6 digits, base to
        // close brace (exclusive of the braces themselves).
        let err = hex_at("{FFFFFF}", 0).unwrap_err();
        let span = err.loc.unwrap();
        assert_eq!(span.lo as usize, 1); // first digit
        assert_eq!(span.hi as usize, 7); // one past last digit, before '}'
    }

    #[test]
    fn hex_does_not_overflow_or_panic_on_many_digits() {
        let err = hex("{FFFFFFFFFFFFFFFF}");
        assert!(err.is_err());
    }

    #[test]
    fn hex_too_many_digits_is_error_even_if_value_would_be_in_range() {
        // 7 digits "0000041" decode to a perfectly valid codepoint
        // (0x41), but the digit *count* alone (1-6 allowed) must still
        // be rejected.
        let err = hex("{0000041}");
        assert!(err.is_err());
        assert_msg_contains(&err.unwrap_err(), "too many hex digits");
    }

    #[test]
    fn hex_exactly_six_digits_is_allowed() {
        // Boundary check: 6 digits is fine even when not "10FFFF" itself,
        // as long as the value is in range.
        assert_eq!(hex("{00FFFF}").unwrap(), 0xFFFF);
    }

    #[test]
    fn hex_seven_digits_is_too_many_regardless_of_value() {
        let err = hex("{1000000}"); // 7 digits, value 0x1000000 also out of range
        assert!(err.is_err());
        // Digit-count error should win (it's detected first, on digit 7,
        // before the value-overflow path would also trigger).
        assert_msg_contains(&err.unwrap_err(), "too many hex digits");
    }

    #[test]
    fn hex_too_many_digits_span_covers_entire_digit_run() {
        // "{0000041}": 7 digits at offsets [1,8), then '}' at 8.
        let err = hex_at("{0000041}", 0).unwrap_err();
        let span = err.loc.unwrap();
        assert_eq!(span.lo as usize, 1);
        assert_eq!(span.hi as usize, 8);
    }

    #[test]
    fn hex_too_many_digits_keeps_consuming_to_closing_brace() {
        // After detecting digit #7, the cursor must still consume the
        // rest of the digits up to '}', leaving the cursor positioned
        // right after '}' (so callers can continue parsing).
        let mut c = cursor("{00000419}rest");
        let err = parse_hex(&mut c);
        assert!(err.is_err());
        assert_eq!(c.peek(), Some(b'r'));
    }

    #[test]
    fn hex_out_of_range_keeps_consuming_to_closing_brace() {
        let mut c = cursor("{FFFFFF}rest");
        let err = parse_hex(&mut c);
        assert!(err.is_err());
        assert_eq!(c.peek(), Some(b'r'));
    }

    #[test]
    fn hex_unterminated_after_too_many_digits_reports_unterminated_not_too_many() {
        // If there's no closing brace at all, even with 7+ digits typed,
        // the EOF/unterminated error should win once we run off the end
        // -- we never reach the point of emitting "too many digits"
        // because we never found the '}' to attach that error to.
        let err = hex("{00000419").unwrap_err();
        assert_msg_contains(&err, "unterminated unicode escape");
    }

    #[test]
    fn hex_cursor_left_after_close_brace() {
        let mut c = cursor("{41}rest");
        let v = parse_hex(&mut c).unwrap();
        assert_eq!(v, 0x41);
        assert_eq!(c.peek(), Some(b'r'));
    }

    // ---------------------------------------------------------------
    // decode_escape
    // ---------------------------------------------------------------

    fn decode(s: &str) -> Result<Escape, Diagnostic> {
        let mut c = cursor(s);
        assert_eq!(c.next(), Some(b'\\'));
        decode_escape(&mut c).map(|sp| sp.esc)
    }

    fn decode_at(s: &str, base: usize) -> Result<Spanned<Escape>, Diagnostic> {
        let mut c = cursor_at(s, base);
        assert_eq!(c.next(), Some(b'\\'));
        decode_escape(&mut c)
    }

    #[test]
    fn decode_simple_escapes() {
        assert!(matches!(decode(r"\n").unwrap(), Escape::Simple('\n')));
        assert!(matches!(decode(r"\t").unwrap(), Escape::Simple('\t')));
        assert!(matches!(decode(r"\r").unwrap(), Escape::Simple('\r')));
        assert!(matches!(decode(r"\\").unwrap(), Escape::Simple('\\')));
        assert!(matches!(decode("\\'").unwrap(), Escape::Simple('\'')));
        assert!(matches!(decode("\\\"").unwrap(), Escape::Simple('"')));
        assert!(matches!(decode(r"\0").unwrap(), Escape::Simple('\0')));
    }

    #[test]
    fn decode_hex_escape_in_bmp_is_simple_char() {
        match decode(r"\x{41}").unwrap() {
            Escape::Simple(c) => assert_eq!(c, 'A'),
        }
    }

    #[test]
    fn decode_hex_escape_returns_simple_for_valid_scalar() {
        match decode(r"\x{41}").unwrap() {
            Escape::Simple(_) => {}
        }
    }

    #[test]
    fn decode_hex_surrogate_is_error() {
        // Surrogate halves are not valid Unicode scalar values and must
        // be rejected at the decode_escape level for both char and string
        // literals.
        assert!(decode(r"\x{D800}").is_err());
        assert!(decode(r"\x{DFFF}").is_err());
        assert!(decode(r"\x{D800}").is_err());
    }

    #[test]
    fn decode_hex_surrogate_error_message() {
        let err = decode(r"\x{D800}").unwrap_err();
        assert_msg_contains(&err, "surrogate");
    }

    #[test]
    fn decode_hex_surrogate_span_covers_full_escape() {
        // '\x{D800}' starting at base 0: span should run from '\' to
        // one past the closing '}', covering the entire escape sequence.
        let spanned = decode_at(r"\x{D800}", 0).unwrap_err();
        let span = spanned.loc.unwrap();
        assert_eq!(span.lo as usize, 0); // the backslash
        assert_eq!(span.hi as usize, 8); // one past '}'
    }

    #[test]
    fn decode_hex_surrogate_boundary_low() {
        // U+D800 is the first surrogate -- must be rejected.
        assert!(decode(r"\x{D800}").is_err());
    }

    #[test]
    fn decode_hex_surrogate_boundary_high() {
        // U+DFFF is the last surrogate -- must be rejected.
        assert!(decode(r"\x{DFFF}").is_err());
    }

    #[test]
    fn decode_hex_just_below_surrogate_range_is_ok() {
        // U+D7FF is the codepoint just before the surrogate range and
        // is a valid scalar value.
        match decode(r"\x{D7FF}").unwrap() {
            Escape::Simple(c) => assert_eq!(c as u32, 0xD7FF),
        }
    }

    #[test]
    fn decode_hex_just_above_surrogate_range_is_ok() {
        // U+E000 is the first codepoint above the surrogate range and
        // is a valid scalar value.
        match decode(r"\x{E000}").unwrap() {
            Escape::Simple(c) => assert_eq!(c as u32, 0xE000),
        }
    }

    #[test]
    fn decode_dangling_backslash_is_error() {
        assert!(decode(r"\").is_err());
    }

    #[test]
    fn decode_unknown_escape_is_error() {
        assert!(decode(r"\q").is_err());
        assert!(decode(r"\1").is_err());
    }

    #[test]
    fn decode_unknown_escape_error_message_lists_valid_escapes() {
        // Spec calls for "as helpful text as possible" -- pin down that
        // the message actually names the valid alternatives, not just
        // "unknown escape".
        let err = decode(r"\q").unwrap_err();
        assert_msg_contains(&err, "\\n");
        assert_msg_contains(&err, "\\x{..}");
    }

    #[test]
    fn decode_unknown_escape_ascii_span_is_two_bytes() {
        // '\' + 'q': span should be esc_start..esc_start+2.
        let spanned = decode_at(r"\q", 10).unwrap_err();
        let span = spanned.loc.unwrap();
        assert_eq!(span.lo as usize, 10);
        assert_eq!(span.hi as usize, 12);
    }

    #[test]
    fn decode_unknown_escape_with_multibyte_char_is_error() {
        // '\é' -- backslash followed directly by a 2-byte UTF-8 char that
        // isn't a recognized escape.
        assert!(decode("\\é").is_err());
    }

    #[test]
    fn decode_unknown_escape_multibyte_spans_full_character() {
        // Regression test: previously `other as char` on a raw
        // continuation byte would corrupt the reported character and
        // potentially mis-span. The span must cover the backslash AND
        // the full multi-byte character, and the message must contain
        // the real decoded char, not garbage.
        let spanned = decode_at("\\é", 0).unwrap_err();
        let span = spanned.loc.unwrap();
        assert_eq!(span.lo as usize, 0); // the backslash
        assert_eq!(span.hi as usize, 3); // '\' (1 byte) + 'é' (2 bytes)
        assert_msg_contains(&spanned, "é");
    }

    #[test]
    fn decode_unknown_escape_four_byte_char_spans_full_character() {
        // '\🦀': backslash (1 byte) + crab emoji (4 bytes).
        let spanned = decode_at("\\🦀", 0).unwrap_err();
        let span = spanned.loc.unwrap();
        assert_eq!(span.lo as usize, 0);
        assert_eq!(span.hi as usize, 5);
        assert_msg_contains(&spanned, "🦀");
    }

    #[test]
    fn decode_propagates_hex_errors() {
        assert!(decode(r"\x41").is_err());
        assert!(decode(r"\x{}").is_err());
        assert!(decode(r"\x{zz}").is_err());
    }

    #[test]
    fn decode_span_covers_whole_escape() {
        let mut c = cursor_at(r"\x{41}", 10);
        c.next();
        let spanned = decode_escape(&mut c).unwrap();
        assert_eq!(spanned.span.lo as usize, 10);
        assert_eq!(spanned.span.hi as usize, 16);
    }

    #[test]
    fn decode_span_for_simple_escape() {
        let mut c = cursor_at(r"\n", 5);
        c.next();
        let spanned = decode_escape(&mut c).unwrap();
        assert_eq!(spanned.span.lo as usize, 5);
        assert_eq!(spanned.span.hi as usize, 7);
    }

    // ---------------------------------------------------------------
    // parse_char (via the real Logos lexer)
    // ---------------------------------------------------------------

    #[test]
    fn char_simple_ascii() {
        assert_eq!(lex_char("'a'").unwrap(), 'a' as u32);
    }

    #[test]
    fn char_simple_escape() {
        assert_eq!(lex_char(r"'\n'").unwrap(), '\n' as u32);
        assert_eq!(lex_char(r"'\t'").unwrap(), '\t' as u32);
        assert_eq!(lex_char(r"'\\'").unwrap(), '\\' as u32);
        assert_eq!(lex_char(r"'\''").unwrap(), '\'' as u32);
    }

    #[test]
    fn char_hex_escape() {
        assert_eq!(lex_char(r"'\x{41}'").unwrap(), 'A' as u32);
        assert_eq!(lex_char(r"'\x{61}'").unwrap(), 'a' as u32);
    }

    #[test]
    fn char_hex_escape_too_many_digits_is_error() {
        assert!(lex_char(r"'\x{0000041}'").is_err());
    }

    #[test]
    fn char_multibyte_utf8_literal() {
        assert_eq!(lex_char("'é'").unwrap(), 'é' as u32);
    }

    #[test]
    fn char_multibyte_utf8_literal_wider() {
        assert_eq!(lex_char("'€'").unwrap(), '€' as u32);
        assert_eq!(lex_char("'🦀'").unwrap(), '🦀' as u32);
    }

    #[test]
    fn char_empty_literal_is_error() {
        assert!(lex_char("''").is_err());
    }

    #[test]
    fn char_empty_literal_error_message() {
        let err = lex_char("''").unwrap_err();
        assert_msg_contains(&err, "empty character literal");
    }

    #[test]
    fn char_dangling_backslash_at_eof_does_not_match_token_regex() {
        // Spec: a char-ish literal with a dangling backslash right at
        // source EOF (`'\` with nothing after it, not even a closing
        // quote) must NOT match the CharLiteral token regex at all --
        // it should never reach `parse_char`/`decode_escape`'s "dangling
        // backslash" error. Confirm no CharLiteral token is produced for
        // this input (either no token at all, or a different token).
        let mut lexer = Token::lexer_with_extras(r"'\", 0);
        let tok = lexer.next();
        match tok {
            Some(Ok(Token::CharLiteral(_))) => {
                panic!("expected '\\ (dangling backslash at EOF) to NOT lex as a CharLiteral")
            }
            Some(Ok(other)) => {
                panic!("expected no match for dangling backslash at EOF, got token {other:?}")
            }
            None | Some(Err(_)) => {} // a lex error (no match) is also acceptable
        }
    }

    #[test]
    fn char_unknown_escape_is_error() {
        assert!(lex_char(r"'\q'").is_err());
    }

    #[test]
    fn char_surrogate_hex_escape_is_error() {
        // Surrogate halves are not valid Unicode scalar values and must
        // be rejected for char literals just as they are for string literals.
        assert!(lex_char(r"'\x{D800}'").is_err());
    }

    #[test]
    fn char_surrogate_hex_escape_error_message() {
        let err = lex_char(r"'\x{D800}'").unwrap_err();
        assert_msg_contains(&err, "surrogate");
    }

    #[test]
    fn char_too_many_characters_span_wraps_entire_literal() {
        // Spec: for "too many characters", the span should cover the
        // *entire* literal including both quotes, not just the tail.
        let mut lexer = Token::lexer_with_extras("'ab'", 0);
        let tok = lexer.next().expect("expected a token");
        let err = tok.expect_err("expected an error for 'ab'");
        let span = err.loc.expect("expected a span");
        assert_eq!(span.lo as usize, 0); // opening quote
        assert_eq!(span.hi as usize, 4); // one past closing quote
    }

    #[test]
    fn char_too_many_characters_with_escape_span_wraps_entire_literal() {
        // Same, but the first character is an escape -- make sure the
        // span still covers the whole literal, not just the part after
        // the escape.
        let mut lexer = Token::lexer_with_extras(r"'\nx'", 0);
        let tok = lexer.next().expect("expected a token");
        let err = tok.expect_err("expected an error for '\\nx'");
        let span = err.loc.expect("expected a span");
        assert_eq!(span.lo as usize, 0);
        assert_eq!(span.hi as usize, 5); // r"'\nx'" is 5 bytes
    }

    #[test]
    fn char_too_many_characters_error_message() {
        let err = lex_char("'ab'").unwrap_err();
        assert_msg_contains(&err, "too many characters");
    }

    // ---------------------------------------------------------------
    // parse_str (via the real Logos lexer)
    // ---------------------------------------------------------------

    #[test]
    fn str_empty() {
        assert_eq!(lex_str(r#""""#).unwrap(), "");
    }

    #[test]
    fn str_plain_ascii() {
        assert_eq!(lex_str(r#""hello""#).unwrap(), "hello");
    }

    #[test]
    fn str_simple_escapes() {
        assert_eq!(lex_str(r#""a\nb\tc""#).unwrap(), "a\nb\tc");
        assert_eq!(lex_str(r#""\\\"""#).unwrap(), "\\\"");
    }

    #[test]
    fn str_hex_escape() {
        assert_eq!(lex_str(r#""\x{41}\x{42}""#).unwrap(), "AB");
    }

    #[test]
    fn str_hex_escape_lowercase_digits() {
        assert_eq!(lex_str(r#""\x{61}""#).unwrap(), "a");
    }

    #[test]
    fn str_hex_escape_too_many_digits_is_error() {
        assert!(lex_str(r#""\x{0000041}""#).is_err());
    }

    #[test]
    fn str_mixed_plain_and_escapes() {
        assert_eq!(lex_str(r#""hi\nthere\x{21}""#).unwrap(), "hi\nthere!");
    }

    #[test]
    fn str_multibyte_utf8_roundtrips() {
        assert_eq!(lex_str(r#""héllo wörld""#).unwrap(), "héllo wörld");
        assert_eq!(lex_str(r#""日本語""#).unwrap(), "日本語");
        assert_eq!(lex_str(r#""🦀🎉""#).unwrap(), "🦀🎉");
    }

    #[test]
    fn str_multibyte_mixed_with_escapes() {
        assert_eq!(lex_str(r#""café\n日本\x{21}""#).unwrap(), "café\n日本!");
    }

    #[test]
    fn str_surrogate_hex_escape_is_error() {
        assert!(lex_str(r#""\x{D800}""#).is_err());
    }

    #[test]
    fn str_surrogate_hex_escape_error_message() {
        let err = lex_str(r#""\x{D800}""#).unwrap_err();
        assert_msg_contains(&err, "surrogate");
    }

    #[test]
    fn str_does_not_splice_surrogate_pairs() {
        // Spec: surrogate halves are never combined, even when they form
        // a valid UTF-16 surrogate pair if you read them together (here,
        // D800 DC00 is the pair that would encode U+10000). Each
        // `\x{...}` is decoded fully independently; the *first* half
        // alone is already an invalid scalar value and must error there
        // -- parse_str must never look ahead to a second `\x{...}` to
        // try to combine them.
        let err = lex_str(r#""\x{D800}\x{DC00}""#).unwrap_err();
        assert_msg_contains(&err, "surrogate");
    }

    #[test]
    fn str_unterminated_hex_escape_is_error() {
        assert!(lex_str(r#""\x{41""#).is_err());
    }

    #[test]
    fn str_unterminated_hex_escape_error_message() {
        // This reaches parse_hex's own EOF handling (see the long
        // comment on the analogous original test): the regex can't match
        // `\x{...}` as a unit without a closing brace, so it falls back
        // to treating `\x` as a generic 2-byte escape and leaves `{41`
        // as plain text for `decode_escape`/`parse_hex` to re-discover
        // and report as unterminated.
        let err = lex_str(r#""\x{41""#).unwrap_err();
        assert_msg_contains(&err, "unterminated unicode escape");
    }

    #[test]
    fn str_unknown_escape_is_error() {
        assert!(lex_str(r#""\q""#).is_err());
    }

    #[test]
    fn str_unknown_escape_with_multibyte_char_is_error() {
        // `"\é"` -- analogous to the decode_escape-level multibyte test,
        // but exercised through the full string-literal path.
        assert!(lex_str("\"\\é\"").is_err());
    }

    #[test]
    fn str_dangling_backslash_at_eof_does_not_match_token_regex() {
        // Spec: same guarantee as for char literals -- a string-ish
        // literal with a dangling backslash right at true source EOF
        // (`"\` with nothing after it) must not match the StrLiteral
        // token regex, so it never reaches `decode_escape`'s "dangling
        // backslash" error path.
        let mut lexer = Token::lexer_with_extras(r#""\"#, 0);
        let tok = lexer.next();
        match tok {
            Some(Ok(Token::StrLiteral(_))) => {
                panic!("expected \"\\ (dangling backslash at EOF) to NOT lex as a StrLiteral")
            }
            Some(Ok(other)) => {
                panic!("expected no match for dangling backslash at EOF, got token {other:?}")
            }
            None | Some(Err(_)) => {}
        }
    }

    #[test]
    fn str_does_not_panic_on_many_consecutive_multibyte_chars() {
        let s = "日本語日本語日本語日本語";
        let src = format!("\"{s}\"");
        let leaked: &'static str = Box::leak(src.into_boxed_str());
        assert_eq!(lex_str(leaked).unwrap(), s);
    }
}
