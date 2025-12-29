use std::{
    fs::File,
    io::{BufReader, Read},
    rc::Rc,
};

use crate::sources::SourceManager;

mod ast;
mod cli;
mod errors;
mod lexer;
mod parser;
mod sources;

fn main() {
    env_logger::init();
    cli::init();
    let mut source_manager = SourceManager::new();
    let sources = &cli::flags().source_files;

    for source in sources {
        let file = File::open(source).expect("failed to open file");
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        reader.read_to_string(&mut buf);

        source_manager.add(
            source.to_str().expect("invalid unicode in file path"),
            Rc::from(buf),
        );
    }

    for id in source_manager.ids() {
        parser::parse(&source_manager, &id);
    }
}
