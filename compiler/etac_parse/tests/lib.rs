#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use expect_test::expect;
use indoc::indoc;

#[macro_use]
pub mod parse;

#[test]
fn use_no_defs() {
    parsetest! {
        "main.eta",
        indoc![r#"
            main() {
                return
            }
        "#],
        expect![[r#"
            main.eta::ast {
            (()
             ((((main @1:1..1:5) () () ((return @2:5..3:1) @1:8..3:2) @1:1..3:2) @1:1..3:2))
             @1:1..3:2)
            }
        "#]]
    }
}

#[test]
fn no_interface_decls() {
    parsetest! {
        "interface.eti",
        indoc![r#"
        "#],
        expect![[r#"
            interface.eti::ast {Failed}
            interface.eti:1:1..1:1 Error {
            	message: Interface must contain at least one method declaration
            	note: 
            	interface.eti:1:1..1:1 "expected declaration"
            }
        "#]]
    }
}

#[test]
fn garbage_definition_recovery() {
    parsetest! {
        "main.eta",
        indoc![r#"
            use io
            )))
        "#],
        expect![[r#"
            main.eta::ast {
            (((use (io @1:5..1:7) @1:1..1:7)) ((Error @2:1..2:4)) @1:1..2:4)
            }
            main.eta:2:1..2:2 Error {
            	message: Unexpected token )
            	note: 
            	main.eta:2:1..2:2 "expected one of \"use\", \";\", or \"ident\""
            }
        "#]]
    }
}

#[test]
fn method_trailing_junk_recovery() {
    parsetest! {
        "main.eta",
        indoc![r#"
            size():int { return 0 }
            }}}
        "#],
        expect![[r#"
            main.eta::ast {
            (()
             ((((size @1:1..1:5)
                ()
                ((int @1:8..1:11))
                ((return (0 @1:21..1:22) @1:14..1:22) @1:12..1:24)
                @1:1..1:24)
               @1:1..1:24)
              (Error @2:1..2:4))
             @1:1..2:4)
            }
            main.eta:2:1..2:2 Error {
            	message: Unexpected token }
            	note: 
            	main.eta:2:1..2:2 "expected one of \"use\", or \"ident\""
            }
        "#]]
    }
}

#[test]
fn malformed_statement_recovery() {
    parsetest! {
        "main.eta",
        indoc![r#"
            main() { + + }
        "#],
        expect![[r#"
            main.eta::ast {
            (()
             ((((main @1:1..1:5) () () ((Error @1:10..1:13) @1:8..1:15) @1:1..1:15)
               @1:1..1:15))
             @1:1..1:15)
            }
            main.eta:1:10..1:11 Error {
            	message: Unexpected token +
            	note: 
            	main.eta:1:10..1:11 "expected one of \"while\", \"if\", \"return\", \"_\", \"{\", \"}\", or \"ident\""
            }
        "#]]
    }
}

#[test]
fn missing_rhs_recovery() {
    parsetest! {
        "main.eta",
        indoc![r#"
            main() { x:int = }
        "#],
        expect![[r#"
            main.eta::ast {
            (()
             ((((main @1:1..1:5)
                ()
                ()
                ((=
                  ((x @1:10..1:11) (int @1:12..1:15) @1:10..1:15)
                  (Error @1:17..1:18)
                  @1:10..1:18)
                 @1:8..1:19)
                @1:1..1:19)
               @1:1..1:19))
             @1:1..1:19)
            }
            main.eta:1:18..1:19 Error {
            	message: Unexpected token }
            	note: 
            	main.eta:1:18..1:19 "expected one of \"length\", \"(\", \"{\", \"!\", \"-\", \"boollit\", \"charlit\", \"strlit\", \"ident\", or \"intlit\""
            }
        "#]]
    }
}

#[test]
fn dangling_binop_recovery() {
    parsetest! {
        "main.eta",
        indoc![r#"
            main() { x:int = a[3 + ] }
        "#],
        expect![[r#"
            main.eta::ast {
            (()
             ((((main @1:1..1:5)
                ()
                ()
                ((=
                  ((x @1:10..1:11) (int @1:12..1:15) @1:10..1:15)
                  ([] ((a @1:18..1:19) @1:18..1:19) (Error @1:20..1:23) @1:18..1:25)
                  @1:10..1:25)
                 @1:8..1:27)
                @1:1..1:27)
               @1:1..1:27))
             @1:1..1:27)
            }
            main.eta:1:24..1:25 Error {
            	message: Unexpected token ]
            	note: 
            	main.eta:1:24..1:25 "expected one of \"length\", \"(\", \"{\", \"!\", \"-\", \"boollit\", \"charlit\", \"strlit\", \"ident\", or \"intlit\""
            }
        "#]]
    }
}
