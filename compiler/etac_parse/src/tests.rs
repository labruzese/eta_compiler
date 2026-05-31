#[cfg(test)]
// This module is a toolkit: some helpers (parse_interface, expect_recovered,
// etc.) exist for tests you'll add later, so don't warn when they're unused.
#[allow(dead_code)]
mod tests {
    use crate::{parse, ParseResult, ProgramParser, InterfaceParser, IParser};
    use etac_errors::{Diagnostic, Level};
    use etac_lexer::Lexer;
    use etac_span::{FileId, Sources};
    use std::io::Write;
    use tempfile::NamedTempFile;

    pub struct Parsed<Out> {
        result: ParseResult<Out>,
        _file: NamedTempFile,
    }

    impl<Out> std::ops::Deref for Parsed<Out> {
        type Target = ParseResult<Out>;
        fn deref(&self) -> &ParseResult<Out> { &self.result }
    }

    impl<Out> Parsed<Out> {
        pub fn error_diags(&self) -> Vec<&Diagnostic> {
            self.result.errors.iter().filter(|d| d.level == Level::Error).collect()
        }
        pub fn error_count(&self) -> usize { self.error_diags().len() }
        pub fn first_error_pos(&self) -> Option<(usize, usize)> {
            let d = self.error_diags().into_iter().find(|d| d.loc.is_some())?;
            let loc = d.loc.as_ref().unwrap();
            let cache = Sources::new();
            cache.lc_index(&loc.file_id, loc.range.start).ok()
        }
        pub fn messages(&self) -> Vec<&str> {
            self.result.errors.iter().map(|d| d.message.as_str()).collect()
        }
    }

    fn run_parse<Out, P: IParser<Out>>(src: &str) -> Parsed<Out> {
        let mut tmp = tempfile::Builder::new()
            .suffix(".eta")
            .tempfile()
            .expect("failed to create temp source file");
        tmp.write_all(src.as_bytes()).expect("failed to write temp source");
        tmp.flush().expect("failed to flush temp source");

        let file_id = FileId::new(tmp.path().to_str().expect("non-utf8 temp path"));
        let cache = Sources::new();
        let source = cache.text(&file_id).expect("temp file should be readable");
        let mut lexer = Lexer::new(&cache, file_id.clone(), &source);
        let result = parse::<_, _, P, _>(&file_id, &mut lexer, &mut |x| x);
        Parsed { result, _file: tmp }
    }

    pub fn parse_program(src: &str) -> Parsed<etac_ast::Program> {
        run_parse::<etac_ast::Program, ProgramParser>(src)
    }
    pub fn parse_interface(src: &str) -> Parsed<etac_ast::Interface> {
        run_parse::<etac_ast::Interface, InterfaceParser>(src)
    }

    #[track_caller]
    pub fn expect_ok(src: &str) -> String {
        let p = parse_program(src);
        assert!(p.is_successful(), "expected clean parse, got {} error(s): {:?}",
            p.errors.len(), p.messages());
        format!("{}", p.output.as_ref().unwrap())
    }
    #[track_caller]
    pub fn expect_recovered(src: &str) -> Parsed<etac_ast::Program> {
        let p = parse_program(src);
        assert!(p.has_recovered(), "expected recovery (output + errors); got output={}, errors={:?}",
            p.output.is_some(), p.messages());
        p
    }
    #[track_caller]
    pub fn expect_failed(src: &str) -> Parsed<etac_ast::Program> {
        let p = parse_program(src);
        assert!(p.has_failed(), "expected hard failure (no output); but parse produced an AST");
        p
    }

    #[test]
    fn use_only_no_definitions_fails() {
        let p = expect_failed("use io");
        assert!(p.error_count() >= 1);
        assert!(p.messages().iter().any(|m| m.contains("at least one definition")));
    }
}
