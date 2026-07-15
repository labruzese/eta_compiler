use etac_cache::{EtaCache, FileId, Span};
use etac_errors::DiagCtxt;
use etac_lexer::Lexer;
use std::fmt::Write as _;

/// testing interface for lexer
pub fn lex<'ec>(cache: &'ec EtaCache, file_id: FileId<'ec>) -> String {
    let dcx = DiagCtxt::new(cache);

    let mut out = String::new();
    for item in Lexer::new(cache.base_offset(file_id), cache.source_text(file_id), &dcx) {
        match item {
            Ok((lo, tok, hi)) => {
                let file = cache.source_name(cache.resolve_span(Span::new(lo, hi)).1);
                let (line_start, column_start) = cache.line_column(lo);
                let (line_end, column_end) = cache.line_column(hi);
                let _ = writeln!(out, "{file}:{line_start}:{column_start}..{line_end}:{column_end} {tok:?}");
            }
            Err(diag) => {
                let loc = diag.loc;
                let level = &diag.level;
                let message = &diag.message;
                let note = diag.note.as_deref().unwrap_or("");
                let mut labels = String::new();
                diag.labels.iter().for_each(|(span, message, ..)| {
                    let file = cache.source_name(cache.resolve_span(*span).1);
                    let (line_start, column_start) = cache.line_column(span.lo);
                    let (line_end, column_end) = cache.line_column(span.hi);
                    let _ = writeln!(labels, "\n\t{file}:{line_start}:{column_start}..{line_end}:{column_end} {message:?}");
                });
                let diag_str = format!(
                    "{level:?} {{\n\tmessage: {message}\n\tnote: {note}{labels}}}"
                );
                match loc {
                    Some(s) => {
                        let file = cache.source_name(cache.resolve_span(s).1);
                        let (line_start, column_start) = cache.line_column(s.lo);
                        let (line_end, column_end) = cache.line_column(s.hi);
                        let _ = writeln!(out, "{file}:{line_start}:{column_start}..{line_end}:{column_end} {diag_str}");
                    }
                    None => {
                        let _ = writeln!(out, "{diag_str}");
                    }
                }
                diag.cancel();
                break;            
            }
        }
    }
    out
}

#[allow(unused_macros)]
macro_rules! lextest {
    // --- single-item shortcuts (braces optional) ---
    ($name:literal, $src:expr, $expected:expr) => {
        lextest! { @wrap $name { $src, $expected } }
    };
    ($src:expr, $expected:expr) => {
        lextest! { @wrap { $src, $expected } }
    };

    // --- @item rules must come BEFORE the general catch-all ---
    (@item $cache:ident, $ids:ident, $n:ident; $name:literal { $src:expr, $expected:expr } $($rest:tt)*) => {
        $ids.push_back($cache.store_source($name, $src.to_owned()).0);
        let expected: expect_test::Expect = $expected;
        expected.assert_eq(&lexer::lex(&$cache, $ids.pop_front().unwrap()));
        lextest!(@item $cache, $ids, $n; $($rest)*);
    };
    (@item $cache:ident, $ids:ident, $n:ident; { $src:expr, $expected:expr } $($rest:tt)*) => {
        let name = format!("src{}", $n);
        $n += 1;
        $ids.push_back($cache.store_source(name, $src.to_owned()).0);
        let expected: expect_test::Expect = $expected;
        expected.assert_eq(&lexer::lex(&$cache, $ids.pop_front().unwrap()));
        lextest!(@item $cache, $ids, $n; $($rest)*);
    };
    (@item $cache:ident, $ids:ident, $n:ident;) => {};

    // --- wrapped/general entry point ---
    (@wrap $($rest:tt)*) => {{
        let cache = etac_cache::EtaCache::new();
        let mut ids = std::collections::VecDeque::new();
        let mut n: usize = 0;
        lextest!(@item cache, ids, n; $($rest)*);
    }};

    // --- top-level catch-all for multi-item / already-braced calls ---
    ($($rest:tt)*) => {
        lextest! { @wrap $($rest)* }
    };
}
