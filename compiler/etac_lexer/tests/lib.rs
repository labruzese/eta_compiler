#![allow(unused_assignments)]

use expect_test::expect;
use indoc::indoc;

#[macro_use]
pub mod lexer;

#[test]
fn add() {
    lextest! {
        "+++",
        expect![[r#"
            src0:1:1..1:2 OperatorAdd
            src0:1:2..1:3 OperatorAdd
            src0:1:3..1:4 OperatorAdd
        "#]]
    }
}
#[test]
fn arrayinit() {
    lextest! {
        indoc![r#"
            a: int[] = {72,101,108,108,111}
            a: int[] = "Hello"
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("a")
            src0:1:2..1:3 OfType
            src0:1:4..1:7 KeywordInt
            src0:1:7..1:8 LBracket
            src0:1:8..1:9 RBracket
            src0:1:10..1:11 Assign
            src0:1:12..1:13 BlockOpen
            src0:1:13..1:15 Integer(72)
            src0:1:15..1:16 Comma
            src0:1:16..1:19 Integer(101)
            src0:1:19..1:20 Comma
            src0:1:20..1:23 Integer(108)
            src0:1:23..1:24 Comma
            src0:1:24..1:27 Integer(108)
            src0:1:27..1:28 Comma
            src0:1:28..1:31 Integer(111)
            src0:1:31..1:32 BlockClose
            src0:2:1..2:2 Identifier("a")
            src0:2:2..2:3 OfType
            src0:2:4..2:7 KeywordInt
            src0:2:7..2:8 LBracket
            src0:2:8..2:9 RBracket
            src0:2:10..2:11 Assign
            src0:2:12..2:19 StrLiteral("Hello")
        "#]]
    }
}
#[test]
fn arrayinit2() {
    lextest! {
        indoc![r#"
            n: int = gcd(10, 2)
            a: int[n]
            while (n > 0) {
              n = n - 1
              a[n] = n
            }
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("n")
            src0:1:2..1:3 OfType
            src0:1:4..1:7 KeywordInt
            src0:1:8..1:9 Assign
            src0:1:10..1:13 Identifier("gcd")
            src0:1:13..1:14 LParen
            src0:1:14..1:16 Integer(10)
            src0:1:16..1:17 Comma
            src0:1:18..1:19 Integer(2)
            src0:1:19..1:20 RParen
            src0:2:1..2:2 Identifier("a")
            src0:2:2..2:3 OfType
            src0:2:4..2:7 KeywordInt
            src0:2:7..2:8 LBracket
            src0:2:8..2:9 Identifier("n")
            src0:2:9..2:10 RBracket
            src0:3:1..3:6 KeywordWhile
            src0:3:7..3:8 LParen
            src0:3:8..3:9 Identifier("n")
            src0:3:10..3:11 RelOpGr
            src0:3:12..3:13 Integer(0)
            src0:3:13..3:14 RParen
            src0:3:15..3:16 BlockOpen
            src0:4:3..4:4 Identifier("n")
            src0:4:5..4:6 Assign
            src0:4:7..4:8 Identifier("n")
            src0:4:9..4:10 Minus
            src0:4:11..4:12 Integer(1)
            src0:5:3..5:4 Identifier("a")
            src0:5:4..5:5 LBracket
            src0:5:5..5:6 Identifier("n")
            src0:5:6..5:7 RBracket
            src0:5:8..5:9 Assign
            src0:5:10..5:11 Identifier("n")
            src0:6:1..6:2 BlockClose
        "#]]
    }
}
#[test]
fn badescape() {
    lextest! {
        indoc![r#"
            This is not a Unicode char: \x{ffffff}
        "#], 
        expect![[r#"
            src0:1:1..1:5 Identifier("This")
            src0:1:6..1:8 Identifier("is")
            src0:1:9..1:12 Identifier("not")
            src0:1:13..1:14 Identifier("a")
            src0:1:15..1:22 Identifier("Unicode")
            src0:1:23..1:27 Identifier("char")
            src0:1:27..1:28 OfType
            src0:1:29..1:30 Error {
            	message: unknown token
            	note: 
            	src0:1:29..1:30 "this token"
            }
        "#]]
    }
}
#[test]
fn beauty() {
    lextest! {
        indoc![r#"
            ===============================================================================
            = This is a beautiful document heading, not a xi program, but it still lexes! =
            ===============================================================================
        "#],
        expect![[r#"
            src0:1:1..1:3 RelOpEq
            src0:1:3..1:5 RelOpEq
            src0:1:5..1:7 RelOpEq
            src0:1:7..1:9 RelOpEq
            src0:1:9..1:11 RelOpEq
            src0:1:11..1:13 RelOpEq
            src0:1:13..1:15 RelOpEq
            src0:1:15..1:17 RelOpEq
            src0:1:17..1:19 RelOpEq
            src0:1:19..1:21 RelOpEq
            src0:1:21..1:23 RelOpEq
            src0:1:23..1:25 RelOpEq
            src0:1:25..1:27 RelOpEq
            src0:1:27..1:29 RelOpEq
            src0:1:29..1:31 RelOpEq
            src0:1:31..1:33 RelOpEq
            src0:1:33..1:35 RelOpEq
            src0:1:35..1:37 RelOpEq
            src0:1:37..1:39 RelOpEq
            src0:1:39..1:41 RelOpEq
            src0:1:41..1:43 RelOpEq
            src0:1:43..1:45 RelOpEq
            src0:1:45..1:47 RelOpEq
            src0:1:47..1:49 RelOpEq
            src0:1:49..1:51 RelOpEq
            src0:1:51..1:53 RelOpEq
            src0:1:53..1:55 RelOpEq
            src0:1:55..1:57 RelOpEq
            src0:1:57..1:59 RelOpEq
            src0:1:59..1:61 RelOpEq
            src0:1:61..1:63 RelOpEq
            src0:1:63..1:65 RelOpEq
            src0:1:65..1:67 RelOpEq
            src0:1:67..1:69 RelOpEq
            src0:1:69..1:71 RelOpEq
            src0:1:71..1:73 RelOpEq
            src0:1:73..1:75 RelOpEq
            src0:1:75..1:77 RelOpEq
            src0:1:77..1:79 RelOpEq
            src0:1:79..1:80 Assign
            src0:2:1..2:2 Assign
            src0:2:3..2:7 Identifier("This")
            src0:2:8..2:10 Identifier("is")
            src0:2:11..2:12 Identifier("a")
            src0:2:13..2:22 Identifier("beautiful")
            src0:2:23..2:31 Identifier("document")
            src0:2:32..2:39 Identifier("heading")
            src0:2:39..2:40 Comma
            src0:2:41..2:44 Identifier("not")
            src0:2:45..2:46 Identifier("a")
            src0:2:47..2:49 Identifier("xi")
            src0:2:50..2:57 Identifier("program")
            src0:2:57..2:58 Comma
            src0:2:59..2:62 Identifier("but")
            src0:2:63..2:65 Identifier("it")
            src0:2:66..2:71 Identifier("still")
            src0:2:72..2:77 Identifier("lexes")
            src0:2:77..2:78 OperatorNot
            src0:2:79..2:80 Assign
            src0:3:1..3:3 RelOpEq
            src0:3:3..3:5 RelOpEq
            src0:3:5..3:7 RelOpEq
            src0:3:7..3:9 RelOpEq
            src0:3:9..3:11 RelOpEq
            src0:3:11..3:13 RelOpEq
            src0:3:13..3:15 RelOpEq
            src0:3:15..3:17 RelOpEq
            src0:3:17..3:19 RelOpEq
            src0:3:19..3:21 RelOpEq
            src0:3:21..3:23 RelOpEq
            src0:3:23..3:25 RelOpEq
            src0:3:25..3:27 RelOpEq
            src0:3:27..3:29 RelOpEq
            src0:3:29..3:31 RelOpEq
            src0:3:31..3:33 RelOpEq
            src0:3:33..3:35 RelOpEq
            src0:3:35..3:37 RelOpEq
            src0:3:37..3:39 RelOpEq
            src0:3:39..3:41 RelOpEq
            src0:3:41..3:43 RelOpEq
            src0:3:43..3:45 RelOpEq
            src0:3:45..3:47 RelOpEq
            src0:3:47..3:49 RelOpEq
            src0:3:49..3:51 RelOpEq
            src0:3:51..3:53 RelOpEq
            src0:3:53..3:55 RelOpEq
            src0:3:55..3:57 RelOpEq
            src0:3:57..3:59 RelOpEq
            src0:3:59..3:61 RelOpEq
            src0:3:61..3:63 RelOpEq
            src0:3:63..3:65 RelOpEq
            src0:3:65..3:67 RelOpEq
            src0:3:67..3:69 RelOpEq
            src0:3:69..3:71 RelOpEq
            src0:3:71..3:73 RelOpEq
            src0:3:73..3:75 RelOpEq
            src0:3:75..3:77 RelOpEq
            src0:3:77..3:79 RelOpEq
            src0:3:79..3:80 Assign
        "#]]
    }
}
#[test]
fn consecutive_operators() {
    lextest! {
        indoc![r#"
            a+-b
            !-x
            --x
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("a")
            src0:1:2..1:3 OperatorAdd
            src0:1:3..1:4 Minus
            src0:1:4..1:5 Identifier("b")
            src0:2:1..2:2 OperatorNot
            src0:2:2..2:3 Minus
            src0:2:3..2:4 Identifier("x")
            src0:3:1..3:2 Minus
            src0:3:2..3:3 Minus
            src0:3:3..3:4 Identifier("x")
        "#]]
    }
}
#[test]
fn empty_hex_escape() {
    lextest! {
        indoc![r#"
            "\x{}"
        "#],
        expect![[r#"
            src0:1:2..1:5 Error {
            	message: empty unicode escape expected non-empty hex between '{{' and '}}'
            	note: 
            	src0:1:2..1:5 "expected non-empty hex here"
            }
        "#]]
    }
}
#[test]
fn error_escape_q() {
    lextest! {
        indoc![r#"
            "\q"
        "#],
        expect![[r#"
            src0:1:2..1:4 Error {
            	message: unknown escape: '\q' is not a recognized escape sequence
            	note: valid escapes: '\n', '\t', '\r', '\\', '\'', '\"', '\0', '\x{..}'
            	src0:1:2..1:4 "this isn't a recognized escape sequence"
            }
        "#]]
    }
}
#[test]
fn error_escape_space() {
    lextest! {
        indoc![r#"
            "\ \ \ "
        "#],
        expect![[r#"
            src0:1:2..1:4 Error {
            	message: unknown escape: '\ ' is not a recognized escape sequence
            	note: valid escapes: '\n', '\t', '\r', '\\', '\'', '\"', '\0', '\x{..}'
            	src0:1:2..1:4 "this isn't a recognized escape sequence"
            }
        "#]]
    }
}
#[test]
fn error_escape_x() {
    lextest! {
        indoc![r#"
            "\x"
        "#],
        expect![[r#"
            src0:1:2..1:4 Error {
            	message: expected '{{' after '\x'
            	note: 
            	src0:1:2..1:4 "at this escape sequence"
            }
        "#]]
    }
}
#[test]
fn escape_double_quote() {
    lextest! {
        indoc![r#"
            '\"'
            "say \"hi\"!"
        "#],
        expect![[r#"
            src0:1:1..1:5 CharLiteral(34)
            src0:2:1..2:14 StrLiteral("say \"hi\"!")
        "#]]
    }
}
#[test]
fn escape_hex_newline() {
    lextest! {
        indoc![r#"
            '\x{0a}'
            "\x{0a}"
        "#],
        expect![[r#"
            src0:1:1..1:9 CharLiteral(10)
            src0:2:1..2:9 StrLiteral("\n")
        "#]]
    }
}
#[test]
fn escape_null_cr() {
    lextest! {
        indoc![r#"
            '\0'
            '\r'
            "\0\r"
        "#],
        expect![[r#"
            src0:1:1..1:5 CharLiteral(0)
            src0:2:1..2:5 CharLiteral(13)
            src0:3:1..3:7 StrLiteral("\0\r")
        "#]]
    }
}
#[test]
fn escapes() {
    lextest! {
        indoc![r#"
            "Hello, Worl\x{64}!" 
            '\x{64}'
            "Hello, Worl\x{b5}!" 
            '\x{b5}'
            "\t"
            '\t'
            "éç"
            "\q" 
        "#],
        expect![[r#"
            src0:1:1..1:21 StrLiteral("Hello, World!")
            src0:2:1..2:9 CharLiteral(100)
            src0:3:1..3:21 StrLiteral("Hello, Worlµ!")
            src0:4:1..4:9 CharLiteral(181)
            src0:5:1..5:5 StrLiteral("\t")
            src0:6:1..6:5 CharLiteral(9)
            src0:7:1..7:7 StrLiteral("éç")
            src0:8:2..8:4 Error {
            	message: unknown escape: '\q' is not a recognized escape sequence
            	note: valid escapes: '\n', '\t', '\r', '\\', '\'', '\"', '\0', '\x{..}'
            	src0:8:2..8:4 "this isn't a recognized escape sequence"
            }
        "#]]
    }
}
#[test]
fn ex1() {
    lextest! {
        indoc![r#"
            use io

            main(args: int[][]) {
              print("Hello, Worl\x{64}!\n")
              c3po: int = 'x' + 47;
              r2d2: int = c3po 
            }
        "#],
        expect![[r#"
            src0:1:1..1:4 KeywordUse
            src0:1:5..1:7 Identifier("io")
            src0:3:1..3:5 Identifier("main")
            src0:3:5..3:6 LParen
            src0:3:6..3:10 Identifier("args")
            src0:3:10..3:11 OfType
            src0:3:12..3:15 KeywordInt
            src0:3:15..3:16 LBracket
            src0:3:16..3:17 RBracket
            src0:3:17..3:18 LBracket
            src0:3:18..3:19 RBracket
            src0:3:19..3:20 RParen
            src0:3:21..3:22 BlockOpen
            src0:4:3..4:8 Identifier("print")
            src0:4:8..4:9 LParen
            src0:4:9..4:31 StrLiteral("Hello, World!\n")
            src0:4:31..4:32 RParen
            src0:5:3..5:7 Identifier("c3po")
            src0:5:7..5:8 OfType
            src0:5:9..5:12 KeywordInt
            src0:5:13..5:14 Assign
            src0:5:15..5:18 CharLiteral(120)
            src0:5:19..5:20 OperatorAdd
            src0:5:21..5:23 Integer(47)
            src0:5:23..5:24 SemiColon
            src0:6:3..6:7 Identifier("r2d2")
            src0:6:7..6:8 OfType
            src0:6:9..6:12 KeywordInt
            src0:6:13..6:14 Assign
            src0:6:15..6:19 Identifier("c3po")
            src0:7:1..7:2 BlockClose
        "#]]
    }
}
#[test]
fn ex2() {
    lextest! {
        indoc![r#"
            x:bool = 4all
            x = ''
            this = does not matter
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("x")
            src0:1:2..1:3 OfType
            src0:1:3..1:7 KeywordBool
            src0:1:8..1:9 Assign
            src0:1:10..1:11 Integer(4)
            src0:1:11..1:14 Identifier("all")
            src0:2:1..2:2 Identifier("x")
            src0:2:3..2:4 Assign
            src0:2:5..2:7 Error {
            	message: empty character literal
            	note: a char literal must contain exactly one character
            	src0:2:5..2:7 "empty here"
            }
        "#]]
    }
}
#[test]
fn gcd() {
    lextest! {
        indoc![r#"
            gcd(a:int, b:int):int {
              while (a != 0) {
                if (a<b) b = b - a
                else a = a - b
              }
              return(b)
            }
        "#],
        expect![[r#"
            src0:1:1..1:4 Identifier("gcd")
            src0:1:4..1:5 LParen
            src0:1:5..1:6 Identifier("a")
            src0:1:6..1:7 OfType
            src0:1:7..1:10 KeywordInt
            src0:1:10..1:11 Comma
            src0:1:12..1:13 Identifier("b")
            src0:1:13..1:14 OfType
            src0:1:14..1:17 KeywordInt
            src0:1:17..1:18 RParen
            src0:1:18..1:19 OfType
            src0:1:19..1:22 KeywordInt
            src0:1:23..1:24 BlockOpen
            src0:2:3..2:8 KeywordWhile
            src0:2:9..2:10 LParen
            src0:2:10..2:11 Identifier("a")
            src0:2:12..2:14 RelOpNeq
            src0:2:15..2:16 Integer(0)
            src0:2:16..2:17 RParen
            src0:2:18..2:19 BlockOpen
            src0:3:5..3:7 KeywordIf
            src0:3:8..3:9 LParen
            src0:3:9..3:10 Identifier("a")
            src0:3:10..3:11 RelOpLt
            src0:3:11..3:12 Identifier("b")
            src0:3:12..3:13 RParen
            src0:3:14..3:15 Identifier("b")
            src0:3:16..3:17 Assign
            src0:3:18..3:19 Identifier("b")
            src0:3:20..3:21 Minus
            src0:3:22..3:23 Identifier("a")
            src0:4:5..4:9 KeywordElse
            src0:4:10..4:11 Identifier("a")
            src0:4:12..4:13 Assign
            src0:4:14..4:15 Identifier("a")
            src0:4:16..4:17 Minus
            src0:4:18..4:19 Identifier("b")
            src0:5:3..5:4 BlockClose
            src0:6:3..6:9 KeywordReturn
            src0:6:9..6:10 LParen
            src0:6:10..6:11 Identifier("b")
            src0:6:11..6:12 RParen
            src0:7:1..7:2 BlockClose
        "#]]
    }
}
#[test]
fn high_mul_operator() {
    lextest! {
        indoc![r#"
            a *>> b
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("a")
            src0:1:3..1:6 OperatorHighMul
            src0:1:7..1:8 Identifier("b")
        "#]]
    }
}
#[test]
fn identifier_with_primes() {
    lextest! {
        indoc![r#"
            x'
            a'b
            x'_1
            q'r's
        "#],
        expect![[r#"
            src0:1:1..1:3 Identifier("x'")
            src0:2:1..2:4 Identifier("a'b")
            src0:3:1..3:5 Identifier("x'_1")
            src0:4:1..4:6 Identifier("q'r's")
        "#]]
    }
}
#[test]
fn insertionsort() {
    lextest! {
        indoc![r#"
            sort(a: int[]) {
              i:int = 0
              n:int = length(a)
              while (i < n) {
                  j:int = i
                  while (j > 0) {
                    if (a[j-1] > a[j]) {
                        swap:int = a[j]
                        a[j] = a[j-1]
                        a[j-1] = swap
                    }
                    j = j-1
                  }
                  i = i+1
              }
            }
        "#],
        expect![[r#"
            src0:1:1..1:5 Identifier("sort")
            src0:1:5..1:6 LParen
            src0:1:6..1:7 Identifier("a")
            src0:1:7..1:8 OfType
            src0:1:9..1:12 KeywordInt
            src0:1:12..1:13 LBracket
            src0:1:13..1:14 RBracket
            src0:1:14..1:15 RParen
            src0:1:16..1:17 BlockOpen
            src0:2:3..2:4 Identifier("i")
            src0:2:4..2:5 OfType
            src0:2:5..2:8 KeywordInt
            src0:2:9..2:10 Assign
            src0:2:11..2:12 Integer(0)
            src0:3:3..3:4 Identifier("n")
            src0:3:4..3:5 OfType
            src0:3:5..3:8 KeywordInt
            src0:3:9..3:10 Assign
            src0:3:11..3:17 KeywordLength
            src0:3:17..3:18 LParen
            src0:3:18..3:19 Identifier("a")
            src0:3:19..3:20 RParen
            src0:4:3..4:8 KeywordWhile
            src0:4:9..4:10 LParen
            src0:4:10..4:11 Identifier("i")
            src0:4:12..4:13 RelOpLt
            src0:4:14..4:15 Identifier("n")
            src0:4:15..4:16 RParen
            src0:4:17..4:18 BlockOpen
            src0:5:7..5:8 Identifier("j")
            src0:5:8..5:9 OfType
            src0:5:9..5:12 KeywordInt
            src0:5:13..5:14 Assign
            src0:5:15..5:16 Identifier("i")
            src0:6:7..6:12 KeywordWhile
            src0:6:13..6:14 LParen
            src0:6:14..6:15 Identifier("j")
            src0:6:16..6:17 RelOpGr
            src0:6:18..6:19 Integer(0)
            src0:6:19..6:20 RParen
            src0:6:21..6:22 BlockOpen
            src0:7:9..7:11 KeywordIf
            src0:7:12..7:13 LParen
            src0:7:13..7:14 Identifier("a")
            src0:7:14..7:15 LBracket
            src0:7:15..7:16 Identifier("j")
            src0:7:16..7:18 Integer(-1)
            src0:7:18..7:19 RBracket
            src0:7:20..7:21 RelOpGr
            src0:7:22..7:23 Identifier("a")
            src0:7:23..7:24 LBracket
            src0:7:24..7:25 Identifier("j")
            src0:7:25..7:26 RBracket
            src0:7:26..7:27 RParen
            src0:7:28..7:29 BlockOpen
            src0:8:13..8:17 Identifier("swap")
            src0:8:17..8:18 OfType
            src0:8:18..8:21 KeywordInt
            src0:8:22..8:23 Assign
            src0:8:24..8:25 Identifier("a")
            src0:8:25..8:26 LBracket
            src0:8:26..8:27 Identifier("j")
            src0:8:27..8:28 RBracket
            src0:9:13..9:14 Identifier("a")
            src0:9:14..9:15 LBracket
            src0:9:15..9:16 Identifier("j")
            src0:9:16..9:17 RBracket
            src0:9:18..9:19 Assign
            src0:9:20..9:21 Identifier("a")
            src0:9:21..9:22 LBracket
            src0:9:22..9:23 Identifier("j")
            src0:9:23..9:25 Integer(-1)
            src0:9:25..9:26 RBracket
            src0:10:13..10:14 Identifier("a")
            src0:10:14..10:15 LBracket
            src0:10:15..10:16 Identifier("j")
            src0:10:16..10:18 Integer(-1)
            src0:10:18..10:19 RBracket
            src0:10:20..10:21 Assign
            src0:10:22..10:26 Identifier("swap")
            src0:11:9..11:10 BlockClose
            src0:12:9..12:10 Identifier("j")
            src0:12:11..12:12 Assign
            src0:12:13..12:14 Identifier("j")
            src0:12:14..12:16 Integer(-1)
            src0:13:7..13:8 BlockClose
            src0:14:7..14:8 Identifier("i")
            src0:14:9..14:10 Assign
            src0:14:11..14:12 Identifier("i")
            src0:14:12..14:13 OperatorAdd
            src0:14:13..14:14 Integer(1)
            src0:15:3..15:4 BlockClose
            src0:16:1..16:2 BlockClose
        "#]]
    }
}
#[test]
fn interface() {
    lextest! {
        indoc![r#"
            
            add(a: int, b: int): int
            matrix(m: int[][]): int[][]
            noreturn(a: int)
            noargs(): int
        "#],
        expect![[r#"
            src0:2:1..2:4 Identifier("add")
            src0:2:4..2:5 LParen
            src0:2:5..2:6 Identifier("a")
            src0:2:6..2:7 OfType
            src0:2:8..2:11 KeywordInt
            src0:2:11..2:12 Comma
            src0:2:13..2:14 Identifier("b")
            src0:2:14..2:15 OfType
            src0:2:16..2:19 KeywordInt
            src0:2:19..2:20 RParen
            src0:2:20..2:21 OfType
            src0:2:22..2:25 KeywordInt
            src0:3:1..3:7 Identifier("matrix")
            src0:3:7..3:8 LParen
            src0:3:8..3:9 Identifier("m")
            src0:3:9..3:10 OfType
            src0:3:11..3:14 KeywordInt
            src0:3:14..3:15 LBracket
            src0:3:15..3:16 RBracket
            src0:3:16..3:17 LBracket
            src0:3:17..3:18 RBracket
            src0:3:18..3:19 RParen
            src0:3:19..3:20 OfType
            src0:3:21..3:24 KeywordInt
            src0:3:24..3:25 LBracket
            src0:3:25..3:26 RBracket
            src0:3:26..3:27 LBracket
            src0:3:27..3:28 RBracket
            src0:4:1..4:9 Identifier("noreturn")
            src0:4:9..4:10 LParen
            src0:4:10..4:11 Identifier("a")
            src0:4:11..4:12 OfType
            src0:4:13..4:16 KeywordInt
            src0:4:16..4:17 RParen
            src0:5:1..5:7 Identifier("noargs")
            src0:5:7..5:8 LParen
            src0:5:8..5:9 RParen
            src0:5:9..5:10 OfType
            src0:5:11..5:14 KeywordInt
        "#]]
    }
}
#[test]
fn int_overflow() {
    lextest! {
        indoc![r#"
            12393498732984798273498
        "#],
        expect![[r#"
            src0:1:1..1:24 Error {
            	message: illegal integer literal: number too large to fit in target type
            	note: eta only supports ints in the range [-2^63, 2^63)
            	src0:1:1..1:24 "this number is too large"
            }
        "#]]
    }
}
#[test]
fn keyword_prefix_identifiers() {
    lextest! {
        indoc![r#"
            integer: int = 0
            boolean: bool = true
            uses: int = 1
            iff: int = 2
            whileLoop: int = 3
            returning: int = 4
            lengths: int = 5
        "#],
        expect![[r#"
            src0:1:1..1:8 Identifier("integer")
            src0:1:8..1:9 OfType
            src0:1:10..1:13 KeywordInt
            src0:1:14..1:15 Assign
            src0:1:16..1:17 Integer(0)
            src0:2:1..2:8 Identifier("boolean")
            src0:2:8..2:9 OfType
            src0:2:10..2:14 KeywordBool
            src0:2:15..2:16 Assign
            src0:2:17..2:21 BoolLiteral(true)
            src0:3:1..3:5 Identifier("uses")
            src0:3:5..3:6 OfType
            src0:3:7..3:10 KeywordInt
            src0:3:11..3:12 Assign
            src0:3:13..3:14 Integer(1)
            src0:4:1..4:4 Identifier("iff")
            src0:4:4..4:5 OfType
            src0:4:6..4:9 KeywordInt
            src0:4:10..4:11 Assign
            src0:4:12..4:13 Integer(2)
            src0:5:1..5:10 Identifier("whileLoop")
            src0:5:10..5:11 OfType
            src0:5:12..5:15 KeywordInt
            src0:5:16..5:17 Assign
            src0:5:18..5:19 Integer(3)
            src0:6:1..6:10 Identifier("returning")
            src0:6:10..6:11 OfType
            src0:6:12..6:15 KeywordInt
            src0:6:16..6:17 Assign
            src0:6:18..6:19 Integer(4)
            src0:7:1..7:8 Identifier("lengths")
            src0:7:8..7:9 OfType
            src0:7:10..7:13 KeywordInt
            src0:7:14..7:15 Assign
            src0:7:16..7:17 Integer(5)
        "#]]
    }
}
#[test]
fn large_int() {
    lextest! {
        indoc![r#"
            1000000000000000000000000000000
        "#],
        expect![[r#"
            src0:1:1..1:32 Error {
            	message: illegal integer literal: number too large to fit in target type
            	note: eta only supports ints in the range [-2^63, 2^63)
            	src0:1:1..1:32 "this number is too large"
            }
        "#]]
    }
}
#[test]
fn leading_zeros() {
    lextest! {
        indoc![r#"
            00
            01
        "#],
        expect![[r#"
            src0:1:1..1:2 Integer(0)
            src0:1:2..1:3 Integer(0)
            src0:2:1..2:2 Integer(0)
            src0:2:2..2:3 Integer(1)
        "#]]
    }
}
#[test]
fn max_int_boundary() {
    lextest! {
        indoc![r#"
            9223372036854775807
            9223372036854775808
        "#],
        expect![[r#"
            src0:1:1..1:20 Integer(9223372036854775807)
            src0:2:1..2:20 Error {
            	message: illegal integer literal: number too large to fit in target type
            	note: eta only supports ints in the range [-2^63, 2^63)
            	src0:2:1..2:20 "this number is too large"
            }
        "#]]
    }
}
#[test]
fn max_valid_codepoint() {
    lextest! {
        indoc![r#"
            '\x{10FFFF}'
        "#],
        expect![[r#"
            src0:1:1..1:13 CharLiteral(1114111)
        "#]]
    }
}
#[test]
fn mdarrays() {
    lextest! {
        indoc![r#"
            a: int[][]
            b: int[3][4]
            a = b
            c: int[3][]
            c[0] = b[0]; c[1] = b[1]; c[2] = b[2]
            d: int[][] = {{1, 0}, {0, 1}}
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("a")
            src0:1:2..1:3 OfType
            src0:1:4..1:7 KeywordInt
            src0:1:7..1:8 LBracket
            src0:1:8..1:9 RBracket
            src0:1:9..1:10 LBracket
            src0:1:10..1:11 RBracket
            src0:2:1..2:2 Identifier("b")
            src0:2:2..2:3 OfType
            src0:2:4..2:7 KeywordInt
            src0:2:7..2:8 LBracket
            src0:2:8..2:9 Integer(3)
            src0:2:9..2:10 RBracket
            src0:2:10..2:11 LBracket
            src0:2:11..2:12 Integer(4)
            src0:2:12..2:13 RBracket
            src0:3:1..3:2 Identifier("a")
            src0:3:3..3:4 Assign
            src0:3:5..3:6 Identifier("b")
            src0:4:1..4:2 Identifier("c")
            src0:4:2..4:3 OfType
            src0:4:4..4:7 KeywordInt
            src0:4:7..4:8 LBracket
            src0:4:8..4:9 Integer(3)
            src0:4:9..4:10 RBracket
            src0:4:10..4:11 LBracket
            src0:4:11..4:12 RBracket
            src0:5:1..5:2 Identifier("c")
            src0:5:2..5:3 LBracket
            src0:5:3..5:4 Integer(0)
            src0:5:4..5:5 RBracket
            src0:5:6..5:7 Assign
            src0:5:8..5:9 Identifier("b")
            src0:5:9..5:10 LBracket
            src0:5:10..5:11 Integer(0)
            src0:5:11..5:12 RBracket
            src0:5:12..5:13 SemiColon
            src0:5:14..5:15 Identifier("c")
            src0:5:15..5:16 LBracket
            src0:5:16..5:17 Integer(1)
            src0:5:17..5:18 RBracket
            src0:5:19..5:20 Assign
            src0:5:21..5:22 Identifier("b")
            src0:5:22..5:23 LBracket
            src0:5:23..5:24 Integer(1)
            src0:5:24..5:25 RBracket
            src0:5:25..5:26 SemiColon
            src0:5:27..5:28 Identifier("c")
            src0:5:28..5:29 LBracket
            src0:5:29..5:30 Integer(2)
            src0:5:30..5:31 RBracket
            src0:5:32..5:33 Assign
            src0:5:34..5:35 Identifier("b")
            src0:5:35..5:36 LBracket
            src0:5:36..5:37 Integer(2)
            src0:5:37..5:38 RBracket
            src0:6:1..6:2 Identifier("d")
            src0:6:2..6:3 OfType
            src0:6:4..6:7 KeywordInt
            src0:6:7..6:8 LBracket
            src0:6:8..6:9 RBracket
            src0:6:9..6:10 LBracket
            src0:6:10..6:11 RBracket
            src0:6:12..6:13 Assign
            src0:6:14..6:15 BlockOpen
            src0:6:15..6:16 BlockOpen
            src0:6:16..6:17 Integer(1)
            src0:6:17..6:18 Comma
            src0:6:19..6:20 Integer(0)
            src0:6:20..6:21 BlockClose
            src0:6:21..6:22 Comma
            src0:6:23..6:24 BlockOpen
            src0:6:24..6:25 Integer(0)
            src0:6:25..6:26 Comma
            src0:6:27..6:28 Integer(1)
            src0:6:28..6:29 BlockClose
            src0:6:29..6:30 BlockClose
        "#]]
    }
}
#[test]
fn modulo_operator() {
    lextest! {
        indoc![r#"
            a % b
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("a")
            src0:1:3..1:4 OperatorMod
            src0:1:5..1:6 Identifier("b")
        "#]]
    }
}
#[test]
fn multiline_string() {
    lextest! {
        indoc![r#"
            "Hello
            World"
        "#],
        expect![[r#"
            src0:1:1..2:7 StrLiteral("Hello\nWorld")
        "#]]
    }
}
#[test]
fn ratadd() {
    lextest! {
        indoc![r#"
            ratadd(p1:int, q1:int, p2:int, q2:int) : (int, int) {
                g:int = gcd(q1,q2)
                p3:int = p1*(q2/g) + p2*(q1/g)
                return (p3, q1/g*q2)
            }
        "#],
        expect![[r#"
            src0:1:1..1:7 Identifier("ratadd")
            src0:1:7..1:8 LParen
            src0:1:8..1:10 Identifier("p1")
            src0:1:10..1:11 OfType
            src0:1:11..1:14 KeywordInt
            src0:1:14..1:15 Comma
            src0:1:16..1:18 Identifier("q1")
            src0:1:18..1:19 OfType
            src0:1:19..1:22 KeywordInt
            src0:1:22..1:23 Comma
            src0:1:24..1:26 Identifier("p2")
            src0:1:26..1:27 OfType
            src0:1:27..1:30 KeywordInt
            src0:1:30..1:31 Comma
            src0:1:32..1:34 Identifier("q2")
            src0:1:34..1:35 OfType
            src0:1:35..1:38 KeywordInt
            src0:1:38..1:39 RParen
            src0:1:40..1:41 OfType
            src0:1:42..1:43 LParen
            src0:1:43..1:46 KeywordInt
            src0:1:46..1:47 Comma
            src0:1:48..1:51 KeywordInt
            src0:1:51..1:52 RParen
            src0:1:53..1:54 BlockOpen
            src0:2:5..2:6 Identifier("g")
            src0:2:6..2:7 OfType
            src0:2:7..2:10 KeywordInt
            src0:2:11..2:12 Assign
            src0:2:13..2:16 Identifier("gcd")
            src0:2:16..2:17 LParen
            src0:2:17..2:19 Identifier("q1")
            src0:2:19..2:20 Comma
            src0:2:20..2:22 Identifier("q2")
            src0:2:22..2:23 RParen
            src0:3:5..3:7 Identifier("p3")
            src0:3:7..3:8 OfType
            src0:3:8..3:11 KeywordInt
            src0:3:12..3:13 Assign
            src0:3:14..3:16 Identifier("p1")
            src0:3:16..3:17 OperatorMul
            src0:3:17..3:18 LParen
            src0:3:18..3:20 Identifier("q2")
            src0:3:20..3:21 OperatorDiv
            src0:3:21..3:22 Identifier("g")
            src0:3:22..3:23 RParen
            src0:3:24..3:25 OperatorAdd
            src0:3:26..3:28 Identifier("p2")
            src0:3:28..3:29 OperatorMul
            src0:3:29..3:30 LParen
            src0:3:30..3:32 Identifier("q1")
            src0:3:32..3:33 OperatorDiv
            src0:3:33..3:34 Identifier("g")
            src0:3:34..3:35 RParen
            src0:4:5..4:11 KeywordReturn
            src0:4:12..4:13 LParen
            src0:4:13..4:15 Identifier("p3")
            src0:4:15..4:16 Comma
            src0:4:17..4:19 Identifier("q1")
            src0:4:19..4:20 OperatorDiv
            src0:4:20..4:21 Identifier("g")
            src0:4:21..4:22 OperatorMul
            src0:4:22..4:24 Identifier("q2")
            src0:4:24..4:25 RParen
            src0:5:1..5:2 BlockClose
        "#]]
    }
}
#[test]
fn lex_pa1_ratadduse() {
    lextest! {
        indoc![r#"
            (p:int, q:int) = ratadd(2, 5, 1, 3)
            (_, q':int) = ratadd(1, 2, 1, 3)
        "#],
        expect![[r#"
            src0:1:1..1:2 LParen
            src0:1:2..1:3 Identifier("p")
            src0:1:3..1:4 OfType
            src0:1:4..1:7 KeywordInt
            src0:1:7..1:8 Comma
            src0:1:9..1:10 Identifier("q")
            src0:1:10..1:11 OfType
            src0:1:11..1:14 KeywordInt
            src0:1:14..1:15 RParen
            src0:1:16..1:17 Assign
            src0:1:18..1:24 Identifier("ratadd")
            src0:1:24..1:25 LParen
            src0:1:25..1:26 Integer(2)
            src0:1:26..1:27 Comma
            src0:1:28..1:29 Integer(5)
            src0:1:29..1:30 Comma
            src0:1:31..1:32 Integer(1)
            src0:1:32..1:33 Comma
            src0:1:34..1:35 Integer(3)
            src0:1:35..1:36 RParen
            src0:2:1..2:2 LParen
            src0:2:2..2:3 Discard
            src0:2:3..2:4 Comma
            src0:2:5..2:7 Identifier("q'")
            src0:2:7..2:8 OfType
            src0:2:8..2:11 KeywordInt
            src0:2:11..2:12 RParen
            src0:2:13..2:14 Assign
            src0:2:15..2:21 Identifier("ratadd")
            src0:2:21..2:22 LParen
            src0:2:22..2:23 Integer(1)
            src0:2:23..2:24 Comma
            src0:2:25..2:26 Integer(2)
            src0:2:26..2:27 Comma
            src0:2:28..2:29 Integer(1)
            src0:2:29..2:30 Comma
            src0:2:31..2:32 Integer(3)
            src0:2:32..2:33 RParen
        "#]]
    }
}
#[test]
fn spec1() {
    lextest! {
        indoc![r#"
            x:int = 2;
            z:int;
            (b: bool, i:int) = f(x}
            s: int[] = "Hello";
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("x")
            src0:1:2..1:3 OfType
            src0:1:3..1:6 KeywordInt
            src0:1:7..1:8 Assign
            src0:1:9..1:10 Integer(2)
            src0:1:10..1:11 SemiColon
            src0:2:1..2:2 Identifier("z")
            src0:2:2..2:3 OfType
            src0:2:3..2:6 KeywordInt
            src0:2:6..2:7 SemiColon
            src0:3:1..3:2 LParen
            src0:3:2..3:3 Identifier("b")
            src0:3:3..3:4 OfType
            src0:3:5..3:9 KeywordBool
            src0:3:9..3:10 Comma
            src0:3:11..3:12 Identifier("i")
            src0:3:12..3:13 OfType
            src0:3:13..3:16 KeywordInt
            src0:3:16..3:17 RParen
            src0:3:18..3:19 Assign
            src0:3:20..3:21 Identifier("f")
            src0:3:21..3:22 LParen
            src0:3:22..3:23 Identifier("x")
            src0:3:23..3:24 BlockClose
            src0:4:1..4:2 Identifier("s")
            src0:4:2..4:3 OfType
            src0:4:4..4:7 KeywordInt
            src0:4:7..4:8 LBracket
            src0:4:8..4:9 RBracket
            src0:4:10..4:11 Assign
            src0:4:12..4:19 StrLiteral("Hello")
            src0:4:19..4:20 SemiColon
        "#]]
    }
}
#[test]
fn spec2() {
    lextest! {
        indoc![r#"
          x = x + 1
          s = {1, 2, 3}
          b = !b
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("x")
            src0:1:3..1:4 Assign
            src0:1:5..1:6 Identifier("x")
            src0:1:7..1:8 OperatorAdd
            src0:1:9..1:10 Integer(1)
            src0:2:1..2:2 Identifier("s")
            src0:2:3..2:4 Assign
            src0:2:5..2:6 BlockOpen
            src0:2:6..2:7 Integer(1)
            src0:2:7..2:8 Comma
            src0:2:9..2:10 Integer(2)
            src0:2:10..2:11 Comma
            src0:2:12..2:13 Integer(3)
            src0:2:13..2:14 BlockClose
            src0:3:1..3:2 Identifier("b")
            src0:3:3..3:4 Assign
            src0:3:5..3:6 OperatorNot
            src0:3:6..3:7 Identifier("b")
        "#]]
    }
}
#[test]
fn spec3() {
    lextest! {
        indoc![r#"
            s: int[] = "Hello" + {13, 10}
        "#],
        expect![[r#"
            src0:1:1..1:2 Identifier("s")
            src0:1:2..1:3 OfType
            src0:1:4..1:7 KeywordInt
            src0:1:7..1:8 LBracket
            src0:1:8..1:9 RBracket
            src0:1:10..1:11 Assign
            src0:1:12..1:19 StrLiteral("Hello")
            src0:1:20..1:21 OperatorAdd
            src0:1:22..1:23 BlockOpen
            src0:1:23..1:25 Integer(13)
            src0:1:25..1:26 Comma
            src0:1:27..1:29 Integer(10)
            src0:1:29..1:30 BlockClose
        "#]]
    }
}
#[test]
fn string_tab() {
    lextest! {
        indoc![r#"
            "\t"
        "#],
        expect![[r#"
            src0:1:1..1:5 StrLiteral("\t")
        "#]]
    }
}
#[test]
fn supplementary_char() {
    lextest! {
        indoc![r#"
            main(args: int[][]) {
                a:int = '😀';
                b:int = '🅰';
                c:int[] = "Hello 😀 World";
            }
        "#],
        expect![[r#"
            src0:1:1..1:5 Identifier("main")
            src0:1:5..1:6 LParen
            src0:1:6..1:10 Identifier("args")
            src0:1:10..1:11 OfType
            src0:1:12..1:15 KeywordInt
            src0:1:15..1:16 LBracket
            src0:1:16..1:17 RBracket
            src0:1:17..1:18 LBracket
            src0:1:18..1:19 RBracket
            src0:1:19..1:20 RParen
            src0:1:21..1:22 BlockOpen
            src0:2:5..2:6 Identifier("a")
            src0:2:6..2:7 OfType
            src0:2:7..2:10 KeywordInt
            src0:2:11..2:12 Assign
            src0:2:13..2:19 CharLiteral(128512)
            src0:2:19..2:20 SemiColon
            src0:3:5..3:6 Identifier("b")
            src0:3:6..3:7 OfType
            src0:3:7..3:10 KeywordInt
            src0:3:11..3:12 Assign
            src0:3:13..3:19 CharLiteral(127344)
            src0:3:19..3:20 SemiColon
            src0:4:5..4:6 Identifier("c")
            src0:4:6..4:7 OfType
            src0:4:7..4:10 KeywordInt
            src0:4:10..4:11 LBracket
            src0:4:11..4:12 RBracket
            src0:4:13..4:14 Assign
            src0:4:15..4:33 StrLiteral("Hello 😀 World")
            src0:4:33..4:34 SemiColon
            src0:5:1..5:2 BlockClose
        "#]]
    }
}
#[test]
fn supplementary_outofbounds() {
    lextest! {
        indoc![r#"
            main(args: int[][]) {
                ok: int[] = "\x{110000}"; 
            }
        "#],
        expect![[r#"
            src0:1:1..1:5 Identifier("main")
            src0:1:5..1:6 LParen
            src0:1:6..1:10 Identifier("args")
            src0:1:10..1:11 OfType
            src0:1:12..1:15 KeywordInt
            src0:1:15..1:16 LBracket
            src0:1:16..1:17 RBracket
            src0:1:17..1:18 LBracket
            src0:1:18..1:19 RBracket
            src0:1:19..1:20 RParen
            src0:1:21..1:22 BlockOpen
            src0:2:5..2:7 Identifier("ok")
            src0:2:7..2:8 OfType
            src0:2:9..2:12 KeywordInt
            src0:2:12..2:13 LBracket
            src0:2:13..2:14 RBracket
            src0:2:15..2:16 Assign
            src0:2:21..2:27 Error {
            	message: unicode escape out of range
            	note: the maximum valid codepoint is U+10FFFF ('\x{10FFFF}')
            	src0:2:21..2:27 "this isn't a valid codepoint"
            }
        "#]]
    }
}
#[test]
fn surrogate_char() {
    lextest! {
        indoc![r#"
            '\x{D800}'
        "#],
        expect![[r#"
            src0:1:2..1:10 Error {
            	message: invalid unicode escape
            	note: U+D800 is a UTF-16 surrogate half and is not a valid Unicode scalar value; surrogate halves (U+D800–U+DFFF) cannot be used in escape sequences
            	src0:1:2..1:10 "this escape produces a surrogate half"
            }
        "#]]
    }
}
#[test]
fn surrogate_string() {
    lextest! {
        indoc![r#"
            "\x{D800}"
        "#],
        expect![[r#"
            src0:1:2..1:10 Error {
            	message: invalid unicode escape
            	note: U+D800 is a UTF-16 surrogate half and is not a valid Unicode scalar value; surrogate halves (U+D800–U+DFFF) cannot be used in escape sequences
            	src0:1:2..1:10 "this escape produces a surrogate half"
            }
        "#]]
    }
}
#[test]
fn unclosedescape() {
    lextest! {
        indoc![r#"
            "Hello, Worl\x{64"
        "#],
        expect![[r#"
            src0:1:13..1:18 Error {
            	message: unterminated unicode escape
            	note: expected a closing '}' before the end of the literal
            	src0:1:13..1:18 "unicode escape unclosed here"
            }
        "#]]
    }
}
#[test]
fn unicode_bmp_char() {
    lextest! {
        indoc![r#"
            'é'
        "#],
        expect![[r#"
            src0:1:1..1:5 CharLiteral(233)
        "#]]
    }
}
