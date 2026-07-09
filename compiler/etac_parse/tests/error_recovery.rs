#![allow(clippy::pedantic)]

use etac_errors::{BufferEmitter, DiagCtxt, Level, RecordedDiag};
use etac_lexer::Lexer;
use etac_parse::{IParser, Parsed};
use etac_span::FileId;
use std::io::Write;
use tempfile::NamedTempFile;

/// Test harness around a single parse: the [`Parsed`] outcome plus the full list of
/// diagnostics the parse emitted (captured by a [`BufferEmitter`]) and the cache
/// needed to resolve their spans. Parsers retain lalrpop's recovered errors until the
/// caller drains them, so the harness emits them through the context -- mirroring the
/// driver -- before snapshotting the buffer.
pub struct Harness<Out> {
    parsed: Parsed<Out>,
    diags: Vec<RecordedDiag>,
    _file: NamedTempFile,
}

impl<Out: std::fmt::Display> Harness<Out> {
    pub fn error_diags(&self) -> Vec<&RecordedDiag> {
        self.diags.iter().filter(|d| d.level == Level::Error).collect()
    }
    pub fn error_count(&self) -> usize {
        self.error_diags().len()
    }
    pub fn first_error_pos(&self) -> Option<(u32, u32)> {
        let d = self.error_diags().into_iter().find(|d| d.loc.is_some())?;
        let loc = d.loc.as_ref().unwrap();
        etac_span::sources().lc_index(loc.lo).ok()
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

fn write_temp_source(src: &str, ext: &str) -> NamedTempFile {
    let mut tmp = tempfile::Builder::new()
        .suffix(ext)
        .tempfile()
        .expect("failed to create temp source file");
    tmp.write_all(src.as_bytes()).expect("failed to write temp source");
    tmp.flush().expect("failed to flush temp source");
    tmp
}

/// Lex and parse `src` with the given parser type (`ProgramParser` or
/// `InterfaceParser`), mirroring the driver's per-file flow. A macro rather than a
/// generic function because the parser is chosen by a constructor that borrows the
/// locally-created diagnostic context.
macro_rules! run_parse {
    ($src:expr, $ext:expr, $parser:ident) => {{
        let tmp = write_temp_source($src, $ext);
        let file_id = FileId::new(tmp.path().to_str().expect("non-utf8 temp path"));

        // Buffer the diagnostics instead of printing them. The buffer is shared
        // with the context (it is an `Rc` inside). Source text lives in the
        // process-global cache; temp files have unique paths, so parallel tests
        // never collide in it.
        let buf = BufferEmitter::new();
        let parsed = {
            let dcx = DiagCtxt::with_emitter(etac_span::sources(), Box::new(buf.clone()));
            let mut spans = etac_ast::SpanTable::new();
            let mut parser = etac_parse::$parser::new(&dcx, &mut spans);
            let (base, source) = etac_span::sources()
                .load(file_id)
                .expect("temp source should load");
            let mut lexer = Lexer::new(base, source, &dcx);
            let parsed = parser.parse(&mut lexer);
            // The parser retains recovered/fatal diagnostics; emit them the way the
            // driver does so they land in the buffer (and no drop-bomb fires).
            for diag in parser.into_errors() {
                let _guar = diag.emit();
            }
            parsed
        };
        let diags = buf.take();

        Harness { parsed, diags, _file: tmp }
    }};
}

#[track_caller]
pub fn expect_ok<Out: std::fmt::Display>(p: Harness<Out>) -> String {
    assert!(
        matches!(p.parsed, Parsed::Ok(..)),
        "expected clean parse, got {} error(s): {:?}",
        p.error_count(),
        p.messages()
    );
    p.output_sexpr().unwrap()
}
#[track_caller]
pub fn expect_recovered<Out: std::fmt::Display>(p: Harness<Out>) -> Harness<Out> {
    assert!(
        matches!(p.parsed, Parsed::Recovered(..)),
        "expected recovery (output + errors); got output={}, errors={:?}",
        p.output_sexpr().is_some(),
        p.messages()
    );
    p
}
#[track_caller]
pub fn expect_failed<Out: std::fmt::Display>(p: Harness<Out>) -> Harness<Out> {
    assert!(
        matches!(p.parsed, Parsed::Failed),
        "expected hard failure (no output); but parse produced an AST"
    );
    p
}

#[test]
fn clean_parse_is_ok() {
    let sexpr = expect_ok(run_parse!("main() { return }", ".eta", ProgramParser));
    assert!(sexpr.contains("main"), "tree should contain the method: {sexpr}");
}

#[test]
fn use_only_no_definitions_fails() {
    let p = expect_failed(run_parse!("use io", ".eta", ProgramParser));
    assert!(p.error_count() >= 1);
    assert!(p.messages().iter().any(|m| m.contains("at least one definition")));
}

#[test]
fn no_method_decls_interface() {
    let p = expect_failed(run_parse!("", ".eti", InterfaceParser));
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
    let p = expect_recovered(run_parse!("use io\n) ) )", ".eta", ProgramParser));
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
    let p = expect_recovered(run_parse!(
        "size():int { return 0 }\n} } }",
        ".eta",
        ProgramParser
    ));
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
    let p = expect_recovered(run_parse!("main() { + + }", ".eta", ProgramParser));
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
    let p = expect_recovered(run_parse!("main() { x:int = }", ".eta", ProgramParser));
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
    let p = expect_recovered(run_parse!("main() { x:int = a[3 + ] }", ".eta", ProgramParser));
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
    let p = expect_failed(run_parse!("main() { x:int = @ }", ".eta", ProgramParser));
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
    let p = expect_recovered(run_parse!("main() { x:int = 3", ".eta", ProgramParser));
    let sexpr = p.output_sexpr().unwrap();
    assert!(
        sexpr.contains("(main () ()"),
        "method signature must survive a broken body: {sexpr}"
    );
    assert!(sexpr.contains("Error"), "the broken body should be marked: {sexpr}");
}

#[test]
fn broken_body_does_not_eat_following_method() {
    let p = expect_recovered(run_parse!("f() { ) ) ) }\ng() { return }", ".eta", ProgramParser));
    let sexpr = p.output_sexpr().unwrap();
    assert!(
        sexpr.contains("(f () () (Error))"),
        "f recovers with its signature: {sexpr}"
    );
    assert!(sexpr.contains("(g () () ((return)))"), "g is untouched: {sexpr}");
}

// Type-level recovery (! => Type::Error)
#[test]
fn broken_param_type_recovers_and_keeps_siblings() {
    // The middle parameter's type annotation is missing. The type-level `!`
    // fires for `b`, but the decl keeps its identifier (`(b Error)`) and the
    // valid params on either side -- plus the whole body -- survive. Without
    // this recovery the entire method would collapse to a Definition::Error.
    let p = expect_recovered(run_parse!(
        "f(a: int, b: , c: bool) { return }",
        ".eta",
        ProgramParser
    ));
    assert_eq!(p.error_count(), 1, "expected exactly one error diagnostic");
    assert_eq!(p.error_node_count(), 1, "only the broken type should become an Error node");

    let sexpr = p.output_sexpr().unwrap();
    assert!(
        sexpr.contains("(f ((a int) (b Error) (c bool)) () ((return)))"),
        "broken param type recovers as Type::Error while siblings and body survive: {sexpr}"
    );
    assert!(p.first_error_pos().is_some(), "error should carry a source location");
}

#[test]
fn broken_return_type_recovers() {
    // The `:` promises a return type but none follows. The type-level `!`
    // fires inside the return-type list, and the method (header + body)
    // survives with a single Type::Error standing in for the return type.
    let p = expect_recovered(run_parse!("f(): { return }", ".eta", ProgramParser));
    assert!(p.error_count() >= 1);
    assert_eq!(p.error_node_count(), 1);

    let sexpr = p.output_sexpr().unwrap();
    assert!(
        sexpr.contains("(f () (Error) ((return)))"),
        "broken return type recovers as Type::Error: {sexpr}"
    );
}

#[test]
fn broken_return_type_keeps_valid_siblings() {
    // A return-type *list* `: , int` whose first element is missing. Recovery
    // is localized to the first slot; the trailing `int` is preserved.
    let p = expect_recovered(run_parse!("f(x: int): , int { return 0 }", ".eta", ProgramParser));
    assert_eq!(p.error_count(), 1);
    assert_eq!(p.error_node_count(), 1);

    let sexpr = p.output_sexpr().unwrap();
    assert!(
        sexpr.contains("(f ((x int)) (Error int) ((return 0)))"),
        "only the broken return-type slot errors; the sibling type survives: {sexpr}"
    );
}

#[test]
fn broken_local_decl_type_does_not_eat_rhs() {
    // A local declaration whose type is missing before an assignment. The
    // type-level `!` must recover *just* the type and stop at `=`, so the
    // right-hand side `5` is still parsed as the assigned value.
    let p = expect_recovered(run_parse!("main() { x: = 5 }", ".eta", ProgramParser));
    assert_eq!(p.error_count(), 1);
    assert_eq!(p.error_node_count(), 1);

    let sexpr = p.output_sexpr().unwrap();
    assert!(
        sexpr.contains("(= (x Error) 5)"),
        "type recovery must not swallow the assignment's RHS: {sexpr}"
    );
}

#[test]
fn broken_array_element_type_recovers() {
    // A parameter declared as an array whose element type is malformed. The
    // type-level `!` collapses the broken type to a single Error node while the
    // method signature and body remain intact.
    let p = expect_recovered(run_parse!("g(x: [ ) { return }", ".eta", ProgramParser));
    assert_eq!(p.error_count(), 1);
    assert_eq!(p.error_node_count(), 1);

    let sexpr = p.output_sexpr().unwrap();
    assert!(
        sexpr.contains("(g ((x Error)) () ((return)))"),
        "a malformed array type recovers as Type::Error: {sexpr}"
    );
}

#[test]
fn clean_types_have_no_error_nodes() {
    // Guard against the type-level `!` firing spuriously on valid input:
    // arrays (sized and unsized), int, and bool must all parse cleanly.
    let sexpr = expect_ok(run_parse!(
        "f(a: int, b: bool, c: int[], d: int[3]): bool { return true }",
        ".eta",
        ProgramParser
    ));
    assert!(!sexpr.contains("Error"), "valid types must not produce Error nodes: {sexpr}");
}

#[test]
fn interface_recovers_bad_declaration() {
    let p = expect_recovered(run_parse!("g():int\nf( ) )", ".eti", InterfaceParser));
    assert!(p.error_count() >= 1);
    assert_eq!(p.error_node_count(), 1);
    let sexpr = p.output_sexpr().unwrap();
    assert!(sexpr.contains("(g () (int))"), "valid decl survives: {sexpr}");
    assert!(sexpr.contains("Error"), "bad decl recovers as an Error item: {sexpr}");
}

#[test]
fn interface_empty_still_hard_fails() {
    // Recovery must NOT mask the "needs at least one declaration" rule.
    let p = expect_failed(run_parse!("", ".eti", InterfaceParser));
    assert!(
        p.messages()
            .iter()
            .any(|m| m.contains("at least one method declaration"))
    );
}
