//! The driver for the compiler
//!
//! Passes input between each phase and attches loggers.
//! Currently also does file resolution / lookup.

use etac_errors::{Diag, DiagCtxt, ErrorGuaranteed, etac_error};
use etac_ast::SpanTable;
use etac_parse::{IParser, Parsed};
use etac_session::{cli::Flags, logger::Logger};
use etac_span::{FileId, Span};
use etac_resolve::{Resolver, File};

pub use crate::status::{CompilationFailure, CompilationSuccess};

mod compat;
mod status;

type Result<T> = std::result::Result<T, ErrorGuaranteed>;
type CompilationResult = std::result::Result<CompilationSuccess, CompilationFailure>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum LoadBlame {
    CommandLine,
    Use(Span),
}

fn load_file(dcx: &DiagCtxt, file: FileId, blame: LoadBlame) -> Result<(u32, &'static str)> {
    match etac_span::sources().load(file) {
        Ok(loaded) => Ok(loaded),
        Err(ioe) => {
            let guar = match blame {
                LoadBlame::CommandLine => Diag::io(dcx, &ioe).emit(),
                LoadBlame::Use(span) => etac_error! {
                    dcx, span, "cannot load interface file `{}`: {}", file.as_str(), ioe;
                    primary: "required by this `use`";
                }.emit(),
            };
            Err(guar)
        }
    }
}



fn parse_one<'dcx, P>(
    logger: &'dcx Logger,
    dcx: &'dcx DiagCtxt,
    file: FileId,
    blame: LoadBlame,
    parser: P,
) -> Result<P::Out>
where
    P: IParser<'dcx, 'static>,
    P::Out: std::fmt::Display,
{
    let (base, source) = load_file(dcx, file, blame)?;

    let lexer = etac_lexer::Lexer::new(base, source, dcx);

    let mut lexer = match (logger.lex, blame) {
        (true, LoadBlame::CommandLine) => compat::ULexer::Tee(logger.tee_lexer(file, etac_span::sources(), lexer)),
        _ => compat::ULexer::Raw(lexer),
    };

    let mut parser = match (logger.parse, blame) {
        (true, LoadBlame::CommandLine) => compat::UParser::Tee(logger.tee_parser(file, etac_span::sources(), parser)),
        _ => compat::UParser::Raw(parser),
    };

    match parser.parse(&mut lexer) {
        Parsed::Ok(tree) => Ok(tree),
        Parsed::Recovered(tree) => {
            let _g: ErrorGuaranteed = parser
                .into_errors()
                .into_iter()
                .map(Diag::emit)
                .reduce(|_, g| g).expect("parse not to recover with no errors");
            Ok(tree)
        },
        Parsed::Failed => {
            let g: ErrorGuaranteed = parser
                .into_errors()
                .into_iter()
                .map(Diag::emit)
                .reduce(|_, g| g).expect("parse not to fail with no errors");

            // emit extra lexical errors
            lexer.for_each(|i| {let _ = i.map_err(Diag::emit);});

            Err(g)
        }
    }
}

pub fn run(flags: &Flags) -> CompilationResult {
    let logger = Logger::new(flags);
    let mut spans = SpanTable::new();
    let dcx = DiagCtxt::new(etac_span::sources());


    let mut resolver = Resolver::new(&flags.source_path, &flags.lib_path);

    let files: Vec<File> = flags
        .source_files
        .iter()
        .filter_map(|file| resolver.classify_cli(&dcx, file))
        .collect();

    let mut programs = Vec::new();
    let mut interfaces = Vec::new();

    for file in files {
        match file {
            File::Program(p) => {
                let pparser = etac_parse::ProgramParser::new(&dcx, &mut spans);
                let program = match parse_one(&logger, &dcx, p, LoadBlame::CommandLine, pparser) {
                    Ok(p) => p,
                    Err(_g) => continue,
                };
                programs.push(program);
            },
            File::Interface(i) => {
                let iparser = etac_parse::InterfaceParser::new(&dcx, &mut spans);
                let interface = match parse_one(&logger, &dcx, i, LoadBlame::CommandLine, iparser) {
                    Ok(i) => i,
                    Err(_g) => continue
                };
                interfaces.push(interface);
            },
        }
    }

    // report errors
    match dcx.has_errors() {
        Some(_)=> Err((&dcx).into()),
        None => Ok((&dcx).into()),
    }
}
