# etac

A compiler for the Eta programming language, written in Rust.

Eta is a small, statically typed imperative language (`int`, `bool`, arrays,
methods, and `use`-based interface files). `etac` currently performs lexical
and syntactic analysis, with an emphasis on good diagnostics and error 
recovery. 

## Build

```sh
cargo build --release
```

## Usage

```sh
etac [--lex] [--parse] [-D <dir>] [--sourcepath <dir>] [--libpath <dir>] <files...>
```

- `--lex` — write a `.lexed` token dump per source file
- `--parse` — write a `.parsed` AST S-expression (or the first syntax error)
- `-D <dir>` — directory for diagnostic output (`-` for stdout)
- `--sourcepath <dir>` — where to resolve relative source paths
- `--libpath <dir>` — where to find `.eti` interface files

Example:

```sh
etac -D - --parse examples/fibonacci.eta
```

## Layout

The workspace lives under `compiler/`: 
- `etac` (CLI),
- `etac_lexer`,
- `etac_parse`,
- `etac_ast`,
- `etac_typecheck`, 

plus the supporting 
- `etac_span`,
- `etac_errors`,
- `etac_session`, 
- `etac_driver`

## Status

Lexing and parsing are complete; type checking is in progress.

## Acknowledgments
This uses resources (the Eta language specification, example programs, and test
cases) from Cornell's CS 4120 Introduction to Compilers:
https://www.cs.cornell.edu/courses/cs4120/2026sp/

This is an independent personal project, I'm not enrolled in the course. 
