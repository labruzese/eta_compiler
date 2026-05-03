mod ast;
mod cli;
mod context;
mod errors;
mod lexer;
mod logger;
mod parser;
mod sources;

use context::Context;
use sources::SourceId;

use crate::sources::{FileId, InterfaceId};

fn main() {
    env_logger::init();

    let mut ctx = Context::new(cli::parse_flags());

    let mut sources: Vec<SourceId> = vec![];
    let mut interfaces: Vec<InterfaceId> = vec![];
    // convert / check sources
    for file in &ctx.flags.source_files {
        let Some(file_str) = file.to_str() else {
                errors::emit(&mut ctx.files, error!(
                    &FileId::new("??"), 
                    0..0usize,
                    "non-UTF8 file name {}", file.to_string_lossy()
                ));
                return;
        };

        match file.extension().map(|x| x.to_str().unwrap()) {
            Some("eta") => sources.push(SourceId::new(file_str)),
            Some("eti") => interfaces.push(InterfaceId::new(file_str)),
            extension => {
                errors::emit(&mut ctx.files, error!(
                    &FileId::new(file_str), 
                    0..0usize,
                    "unknown file type {}", extension.unwrap_or("")
                ));
            }
        }
    }

    // parse interfaces
    for file_id in &interfaces {
        match parser::parse_interface(&mut ctx, file_id) {
            Ok(_interface) => {}
            Err(diags) => {
                for d in diags {
                    errors::emit(&mut ctx.files, d);
                }
            }
        }
    }

    // parse sources
    for file_id in &sources {
        match parser::parse_source(&mut ctx, file_id) {
            Ok(_program) => {}
            Err(diags) => {
                for d in diags {
                    errors::emit(&mut ctx.files, d);
                }
            }
        }
    }
}
