use etac_cache::{EtaCache, FileId};
use etac_errors::DiagCtxt;
use etac_lexer::Lexer;
use etac_parse::{IParser, InterfaceParser, ProgramParser};

pub enum FileType {
    Eta,
    Eti,
}

/// testing interface for parser
pub fn parse<'ec>(etype: FileType, cache: &'ec EtaCache, file_id: FileId<'ec>) -> String {
    let dcx = DiagCtxt::new(cache);
    let mut lexer = Lexer::new(cache.base_offset(file_id), cache.source_text(file_id), &dcx);
    let mut out = String::new();
    match etype {
        FileType::Eta => {
            let mut parser = ProgramParser::new(&dcx);
            let parse = parser.parse(&mut lexer);
            etac_test::write_parse_output(&mut out, file_id, parse, cache);
            for error in parser.into_errors() {
                etac_test::write_diag(&mut out, error, cache)
            }
        }
        FileType::Eti => {
            let mut parser = InterfaceParser::new(&dcx);
            let parse = parser.parse(&mut lexer);
            etac_test::write_parse_output(&mut out, file_id, parse, cache);
            for error in parser.into_errors() {
                etac_test::write_diag(&mut out, error, cache)
            }
        }
    }

    out
}

#[allow(unused_macros)]
macro_rules! parsetest {
    // --- single-item shortcuts (braces optional) ---
    ($name:literal, $src:expr, $expected:expr) => {
        parsetest! { @wrap $name { $src, $expected } }
    };

    // --- @item rules must come BEFORE the general catch-all ---
    (@item $cache:ident, $ids:ident, $n:ident; $name:literal { $src:expr, $expected:expr } $($rest:tt)*) => {
        $ids.push_back($cache.store_source(String::from($name), $src.to_owned()).0);
        const NAME: &str = $name;
        const EXPECTED: expect_test::Expect = $expected;
        const ETYPE: parse::FileType = crate::parse::file_type_from_name(NAME);
        EXPECTED.assert_eq(&parse::parse(ETYPE, &$cache, $ids.pop_front().unwrap()));
        parsetest!(@item $cache, $ids, $n; $($rest)*);
    };
    (@item $cache:ident, $ids:ident, $n:ident;) => {};

    // --- wrapped/general entry point ---
    (@wrap $($rest:tt)*) => {{
        let cache = etac_cache::EtaCache::new();
        let mut ids = std::collections::VecDeque::new();
        let mut n: usize = 0;
        parsetest!(@item cache, ids, n; $($rest)*);
    }};

    // --- top-level catch-all for multi-item / already-braced calls ---
    ($($rest:tt)*) => {
        parsetest! { @wrap $($rest)* }
    };
}

pub(crate) const fn ext_eq(bytes: &[u8], start: usize, end: usize, ext: &[u8]) -> bool {
    if end - start != ext.len() {
        return false;
    }
    let mut i = 0;
    while i < ext.len() {
        if bytes[start + i] != ext[i] {
            return false;
        }
        i += 1;
    }
    true
}

pub(crate) const fn file_type_from_name(name: &str) -> FileType {
    let bytes = name.as_bytes();
    let len = bytes.len();

    // find last '.'
    let mut i = len;
    let mut dot = None;
    while i > 0 {
        i -= 1;
        if bytes[i] == b'.' {
            dot = Some(i);
            break;
        }
    }

    match dot {
        Some(idx) => {
            if ext_eq(bytes, idx + 1, len, b"eta") {
                FileType::Eta
            } else if ext_eq(bytes, idx + 1, len, b"eti") {
                FileType::Eti
            } else {
                panic!("not a recognized etac filetype: try one of [eta, eti]")
            }
        }
        None => panic!("file name has no extension"),
    }
}
