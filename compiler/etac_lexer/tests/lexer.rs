use etac_cache::{EtaCache, FileId};
use etac_errors::DiagCtxt;
use etac_lexer::Lexer;

/// testing interface for lexer
pub fn lex<'ec>(cache: &'ec EtaCache, file_id: FileId<'ec>) -> String {
    let dcx = DiagCtxt::new(cache);

    let mut out = String::new();
    for item in Lexer::new(cache.base_offset(file_id), cache.source_text(file_id), &dcx) {
        match item {
            Ok(token) => {
                etac_test::write_token(&mut out, token, cache);
            }
            Err(diag) => {
                etac_test::write_diag(&mut out, diag, cache);
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
