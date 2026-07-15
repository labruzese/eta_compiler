//! The driver for the compiler
//!
//! Creates the compilation's [`EtaCache`] and diagnostic context, then passes
//! input between each phase, storing every phase's output back into the cache
//! and attaching loggers along the way.

use etac_cache::{EtaCache, FileId};
use etac_errors::{Diag, DiagCtxt, ErrorGuaranteed};
use etac_parse::{IParser, Parsed};
use etac_resolve::{File, Resolver};
use etac_session::{cli::Flags, logger::Logger};

pub use crate::status::{CompilationFailure, CompilationSuccess};

mod compat;
mod status;

type Result<T> = std::result::Result<T, ErrorGuaranteed>;
type CompilationResult = std::result::Result<CompilationSuccess, CompilationFailure>;

fn parse_one<'dcx, P>(
    logger: &Logger,
    dcx: &'dcx DiagCtxt<'dcx>,
    file: FileId<'dcx>,
    parser: P,
) -> Result<P::Out>
where
    P: IParser<'dcx, 'dcx>,
    P::Out: std::fmt::Display,
{
    let cache = dcx.cache();
    let lexer = etac_lexer::Lexer::new(cache.base_offset(file), cache.source_text(file), dcx);

    let mut lexer = if logger.lex {
        compat::ULexer::Tee(logger.tee_lexer(file, cache, lexer))
    } else {
        compat::ULexer::Raw(lexer)
    };

    let mut parser = if logger.parse {
        compat::UParser::Tee(logger.tee_parser(file, cache, parser))
    } else {
        compat::UParser::Raw(parser)
    };

    match parser.parse(&mut lexer) {
        Parsed::Ok(tree) => Ok(tree),
        Parsed::Recovered(tree) => {
            let _g: ErrorGuaranteed = parser
                .into_errors()
                .into_iter()
                .map(Diag::emit)
                .reduce(|_, g| g)
                .expect("parse not to recover with no errors");
            Ok(tree)
        }
        Parsed::Failed => {
            let g: ErrorGuaranteed = parser
                .into_errors()
                .into_iter()
                .map(Diag::emit)
                .reduce(|_, g| g)
                .expect("parse not to fail with no errors");

            // drain the remaining token stream so trailing lexical errors still reach the user
            lexer.for_each(|i| {
                let _ = i.map_err(Diag::emit);
            });

            Err(g)
        }
    }
}

pub fn run(flags: &Flags) -> CompilationResult {
    let logger = Logger::new(flags);
    let cache = EtaCache::new();
    let dcx = DiagCtxt::new(&cache);

    let mut resolver = Resolver::new(&flags.source_path, &flags.lib_path);

    let files: Vec<File<'_>> = flags
        .source_files
        .iter()
        .filter_map(|path| match resolver.classify_cli(&dcx, path) {
            Ok(file) => file,
            Err(diag) => {
                diag.emit();
                None
            }
        })
        .collect();

    for file in files {
        match file {
            File::Program(id) => {
                if let Ok(program) = parse_one(&logger, &dcx, id, etac_parse::ProgramParser::new(&dcx)) {
                    cache.store_program(id, program);
                }
            }
            File::Interface(id) => {
                if let Ok(interface) = parse_one(&logger, &dcx, id, etac_parse::InterfaceParser::new(&dcx)) {
                    cache.store_interface(id, interface);
                }
            }
        }
    }

    match dcx.has_errors() {
        Some(_) => Err((&dcx).into()),
        None => Ok((&dcx).into()),
    }
}
