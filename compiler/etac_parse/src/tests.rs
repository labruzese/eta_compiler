#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use crate::{parse, IParser, InterfaceParser, ParseResult, ProgramParser};
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
        fn deref(&self) -> &ParseResult<Out> {
            &self.result
        }
    }

    impl<Out: std::fmt::Display> Parsed<Out> {
        pub fn error_diags(&self) -> Vec<&Diagnostic> {
            self.result.errors.iter().filter(|d| d.level == Level::Error).collect()
        }
        pub fn error_count(&self) -> usize {
            self.error_diags().len()
        }
        pub fn first_error_pos(&self) -> Option<(usize, usize)> {
            let d = self.error_diags().into_iter().find(|d| d.loc.is_some())?;
            let loc = d.loc.as_ref().unwrap();
            let cache = Sources::new();
            cache.lc_index(&loc.file_id, loc.range.start).ok()
        }
        pub fn messages(&self) -> Vec<&str> {
            self.result.errors.iter().map(|d| d.message.as_str()).collect()
        }
        pub fn output_sexpr(&self) -> Option<String> {
            self.result.output.as_ref().map(|o| format!("{o}"))
        }
        pub fn error_node_count(&self) -> usize {
            self.output_sexpr().map(|s| s.split("Error").count() - 1).unwrap_or(0)
        }
    }

    fn run_parse<Out, P: IParser<Out>>(src: &str, ext: &str) -> Parsed<Out> {
        let mut tmp = tempfile::Builder::new()
            .suffix(ext)
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

    #[track_caller]
    pub fn expect_ok<Out: std::fmt::Display, P: IParser<Out>>(src: &str, ext: &str) -> String {
        let p = run_parse::<_, P>(src, ext);
        assert!(
            p.is_successful(),
            "expected clean parse, got {} error(s): {:?}",
            p.errors.len(),
            p.messages()
        );
        format!("{}", p.output.as_ref().unwrap())
    }
    #[track_caller]
    pub fn expect_recovered<Out: std::fmt::Display, P: IParser<Out>>(src: &str, ext: &str) -> Parsed<Out> {
        let p = run_parse::<_, P>(src, ext);
        assert!(
            p.has_recovered(),
            "expected recovery (output + errors); got output={}, errors={:?}",
            p.output.is_some(),
            p.messages()
        );
        p
    }
    #[track_caller]
    pub fn expect_failed<Out: std::fmt::Display, P: IParser<Out>>(src: &str, ext: &str) -> Parsed<Out> {
        let p = run_parse::<_, P>(src, ext);
        assert!(
            p.has_failed(),
            "expected hard failure (no output); but parse produced an AST"
        );
        p
    }

    #[test]
    fn use_only_no_definitions_fails() {
        let p = expect_failed::<_, ProgramParser>("use io", "eta");
        assert!(p.error_count() >= 1);
        assert!(p.messages().iter().any(|m| m.contains("at least one definition")));
    }

    #[test]
    fn no_method_decls_interface() {
        let p = expect_failed::<_, InterfaceParser>("", "eti");
        assert!(p.error_count() >= 1);
        assert!(p
            .messages()
            .iter()
            .any(|m| m.contains("at least one method declaration")));
    }

    // Definition-level recovery (! => Definition::Error)
    #[test]
    fn garbage_definition_recovers_as_error_node() {
        todo!()
    }
    #[test]
    fn trailing_garbage_after_valid_method_recovers() {
        todo!()
    }

    // Statement-level recovery (! => Stmt::Error)
    #[test]
    fn bad_statement_becomes_error_stmt() {
        todo!()
    }

    // Expression-level recovery (! => Expr::Error)
    #[test]
    fn missing_rhs_expression_becomes_error_expr() {
        todo!()
    }
    #[test]
    fn dangling_binary_operator_recovers() {
        todo!()
    }

    // Lexical errors abort BEFORE grammar recovery
    #[test]
    fn unknown_character_is_a_hard_failure_not_recovery() {
        todo!()
    }
}
