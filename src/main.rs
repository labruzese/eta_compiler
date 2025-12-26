use std::{path::PathBuf, rc::Rc};

mod ast;
mod errors;
mod flags;
mod lexer;
mod parser;
mod sources;

fn main() {
    env_logger::init();
    flags::init_flags(flags::Flags {
        lex: true,
        lex_file: PathBuf::from("lexed.lex"),
    });
    let mut sm = sources::SourceManager::new();
    let source = r#"p2*(#&!q1/g)"#;
    let fid = sm.add("dummy.rs", Rc::from(source));
    parser::parse(sm, fid);
}
