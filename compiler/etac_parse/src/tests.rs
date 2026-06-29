#[cfg(test)]
mod tests {
    use crate::{IParser, InterfaceParser, Parsed, ProgramParser, parse};
    use etac_errors::{BufferEmitter, DiagCtxt, Diagnostic, Level};
    use etac_lexer::Lexer;
    use etac_span::{FileId, SourceCache};
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Test harness around a single parse: the [`Parsed`] outcome plus the full list of
    /// diagnostics the parse emitted (captured by a [`BufferEmitter`]) and the cache
    /// needed to resolve their spans. The `parse` entry point emits through a context
    /// rather than returning a diagnostic list, so we snapshot the buffer here.
    pub struct Harness<Out> {
        parsed: Parsed<Out>,
        diags: Vec<Diagnostic>,
        _file: NamedTempFile,
        cache: SourceCache,
    }

    impl<Out: std::fmt::Display> Harness<Out> {
        pub fn error_diags(&self) -> Vec<&Diagnostic> {
            self.diags.iter().filter(|d| d.level == Level::Error).collect()
        }
        pub fn error_count(&self) -> usize {
            self.error_diags().len()
        }
        pub fn first_error_pos(&self) -> Option<(usize, usize)> {
            let d = self.error_diags().into_iter().find(|d| d.loc.is_some())?;
            let loc = d.loc.as_ref().unwrap();
            self.cache.lc_index(loc.lo).ok()
        }
        pub fn messages(&self) -> Vec<&str> {
            self.error_diags().iter().map(|d| d.message.as_str()).collect()
        }
        pub fn output_sexpr(&self) -> Option<String> {
            self.parsed.output().map(|o| format!("{o}"))
        }
        pub fn error_node_count(&self) -> usize {
            self.output_sexpr().map(|s| s.split("Error").count() - 1).unwrap_or(0)
        }
    }

    fn run_parse<Out, P: IParser<Out>>(src: &str, ext: &str) -> Harness<Out> {
        let mut tmp = tempfile::Builder::new()
            .suffix(ext)
            .tempfile()
            .expect("failed to create temp source file");
        tmp.write_all(src.as_bytes()).expect("failed to write temp source");
        tmp.flush().expect("failed to flush temp source");

        let file_id = FileId::new(tmp.path().to_str().expect("non-utf8 temp path"));
        let cache = SourceCache::new();
        let (base, source) = cache.load(&file_id).unwrap();
        let mut lexer = Lexer::new(base, &source);

        // Buffer the diagnostics instead of printing them. The buffer is shared with the
        // context (it is an `Rc` inside); dropping the context at the end of the block
        // releases its borrow of `cache` so the cache can move into the harness.
        let buf = BufferEmitter::new();
        let parsed = {
            let dcx = DiagCtxt::with_emitter(&cache, Box::new(buf.clone()));
            parse::<_, _, P>(&dcx, &mut lexer)
        };
        let diags = buf.take();

        Harness { parsed, diags, _file: tmp, cache }
    }

    #[track_caller]
    #[allow(dead_code)]
    pub fn expect_ok<Out: std::fmt::Display + std::fmt::Debug, P: IParser<Out>>(src: &str, ext: &str) -> String {
        let p = run_parse::<_, P>(src, ext);
        assert!(
            matches!(p.parsed, Parsed::Ok(..)),
            "expected clean parse, got {} error(s): {:?}",
            p.error_count(),
            p.messages()
        );
        p.output_sexpr().unwrap()
    }
    #[track_caller]
    pub fn expect_recovered<Out: std::fmt::Display + std::fmt::Debug, P: IParser<Out>>(src: &str, ext: &str) -> Harness<Out> {
        let p = run_parse::<_, P>(src, ext);
        assert!(
            matches!(p.parsed, Parsed::Recovered { .. }),
            "expected recovery (output + errors); got output={}, errors={:?}",
            p.output_sexpr().is_some(),
            p.messages()
        );
        p
    }
    #[track_caller]
    pub fn expect_failed<Out: std::fmt::Display + std::fmt::Debug, P: IParser<Out>>(src: &str, ext: &str) -> Harness<Out> {
        let p = run_parse::<_, P>(src, ext);
        assert!(
            matches!(p.parsed, Parsed::Failed { .. }),
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
        assert!(
            p.messages()
                .iter()
                .any(|m| m.contains("at least one method declaration"))
        );
    }

    // Definition-level recovery (! => Definition::Error)
    #[test]
    fn garbage_definition_recovers_as_error_node() {
        // The use-list is valid, but the only "definition" is junk tokens.
        // The `! => Definition::Error` production should swallow them, satisfy
        // `Definition+`, and let the parse produce an AST alongside a diagnostic.
        let p = expect_recovered::<_, ProgramParser>("use io\n) ) )", "eta");
        assert_eq!(p.error_count(), 1, "expected exactly one error diagnostic");
        assert_eq!(
            p.error_node_count(),
            1,
            "the garbage should collapse into a single Definition::Error node"
        );

        let sexpr = p.output_sexpr().unwrap();
        // The valid use survived and the lone definition is the error node.
        assert!(sexpr.contains("(use io)"), "use should be preserved: {sexpr}");
        assert!(
            sexpr.contains("(Error)"),
            "definitions list should hold the error: {sexpr}"
        );
        assert!(
            p.messages().iter().any(|m| m.contains("Unexpected token")),
            "diagnostic should report the unexpected token: {:?}",
            p.messages()
        );
    }
    #[test]
    fn trailing_garbage_after_valid_method_recovers() {
        // A fully valid method followed by stray closing braces. Recovery should
        // keep the method intact and only the trailing junk becomes an error node.
        let p = expect_recovered::<_, ProgramParser>("size():int { return 0 }\n} } }", "eta");
        assert!(p.error_count() >= 1);
        assert_eq!(p.error_node_count(), 1, "only the trailing garbage should error");

        let sexpr = p.output_sexpr().unwrap();
        // The good method (header, return type, and body) is untouched...
        assert!(
            sexpr.contains("(size () (int) ((return 0)))"),
            "valid method must be preserved verbatim: {sexpr}"
        );
        // ...and the recovery node sits *after* it as a sibling definition.
        let method_pos = sexpr.find("(size").unwrap();
        let error_pos = sexpr.find("Error").unwrap();
        assert!(error_pos > method_pos, "error node should trail the method: {sexpr}");
    }

    // Statement-level recovery (! => Stmt::Error)
    #[test]
    fn bad_statement_becomes_error_stmt() {
        // `+ +` is not a valid statement (the `+` operators have no operands and
        // `+` is not a unary op), so the statement-level `!` fires.
        let p = expect_recovered::<_, ProgramParser>("main() { + + }", "eta");
        assert!(p.error_count() >= 1);
        assert_eq!(p.error_node_count(), 1);

        let sexpr = p.output_sexpr().unwrap();
        // The method header survives; the body is a single Error statement.
        assert!(
            sexpr.contains("(main () () (Error))"),
            "bad statement should recover as Stmt::Error inside the body: {sexpr}"
        );
        assert!(p.first_error_pos().is_some(), "error should carry a source location");
    }

    // Expression-level recovery (! => Expr::Error)
    #[test]
    fn missing_rhs_expression_becomes_error_expr() {
        // Assignment with nothing on the right-hand side: the expression-level
        // `!` fires for the missing value, but the target decl still parses.
        let p = expect_recovered::<_, ProgramParser>("main() { x:int = }", "eta");
        assert!(p.error_count() >= 1);
        assert_eq!(p.error_node_count(), 1);

        let sexpr = p.output_sexpr().unwrap();
        // Target `(x int)` is preserved; the value collapses to Expr::Error.
        assert!(
            sexpr.contains("(= (x int) Error)"),
            "missing RHS should recover as an Expr::Error value: {sexpr}"
        );
    }
    #[test]
    fn dangling_binary_operator_recovers() {
        // A binary `+` with no right operand, nested inside an index expression.
        // Recovery is localized to the missing operand: the surrounding index
        // (`a[...]`) and its base survive while only the operand is Expr::Error.
        let p = expect_recovered::<_, ProgramParser>("main() { x:int = a[3 + ] }", "eta");
        assert!(p.error_count() >= 1);
        assert_eq!(p.error_node_count(), 1);

        let sexpr = p.output_sexpr().unwrap();
        // The index structure and its array base `a` are intact around the error.
        assert!(
            sexpr.contains("([] a Error)"),
            "dangling operator should recover with surrounding expr intact: {sexpr}"
        );
    }

    // Lexical errors abort BEFORE grammar recovery
    #[test]
    fn unknown_character_is_a_hard_failure_not_recovery() {
        // `@` is not a lexable token. The lexer yields an Err, which surfaces as
        // a ParseError::User and aborts the parse entirely -- the grammar's `!`
        // recovery rules never get a chance to run, so there is no AST.
        let p = expect_failed::<_, ProgramParser>("main() { x:int = @ }", "eta");
        assert!(p.output_sexpr().is_none(), "a lexical error must not yield an AST");
        assert_eq!(
            p.error_node_count(),
            0,
            "no Error recovery nodes should exist on a hard lexical failure"
        );
        assert!(
            p.messages().iter().any(|m| m.contains("unknown token")),
            "failure should come from the lexer, not the grammar: {:?}",
            p.messages()
        );
    }

    #[test]
    fn broken_body_preserves_method_signature() {
        let p = expect_recovered::<_, ProgramParser>("main() { x:int = 3", "eta");
        let sexpr = p.output_sexpr().unwrap();
        assert!(
            sexpr.contains("(main () ()"),
            "method signature must survive a broken body: {sexpr}"
        );
        assert!(sexpr.contains("Error"), "the broken body should be marked: {sexpr}");
    }

    #[test]
    fn broken_body_does_not_eat_following_method() {
        let p = expect_recovered::<_, ProgramParser>("f() { ) ) ) }\ng() { return }", "eta");
        let sexpr = p.output_sexpr().unwrap();
        assert!(
            sexpr.contains("(f () () (Error))"),
            "f recovers with its signature: {sexpr}"
        );
        assert!(sexpr.contains("(g () () ((return)))"), "g is untouched: {sexpr}");
    }

    #[test]
    fn interface_recovers_bad_declaration() {
        let p = expect_recovered::<_, InterfaceParser>("g():int\nf( ) )", "eti");
        assert!(p.error_count() >= 1);
        assert_eq!(p.error_node_count(), 1);
        let sexpr = p.output_sexpr().unwrap();
        assert!(sexpr.contains("(g () (int))"), "valid decl survives: {sexpr}");
        assert!(sexpr.contains("Error"), "bad decl recovers as an Error item: {sexpr}");
    }

    #[test]
    fn interface_empty_still_hard_fails() {
        // Recovery must NOT mask the "needs at least one declaration" rule.
        let p = expect_failed::<_, InterfaceParser>("", "eti");
        assert!(
            p.messages()
                .iter()
                .any(|m| m.contains("at least one method declaration"))
        );
    }
}
