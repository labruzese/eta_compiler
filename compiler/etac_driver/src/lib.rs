//! The driver for the compiler
//!
//! Is responsible for passing input between each phase and attaching the
//! --lex, --parse, etc. loggers to each phase.
//!
//! Every diagnostic in the pipeline flows through a single [`DiagCtxt`] created here; the
//! driver never collects a `Vec<Diagnostic>` to drain. Logging is attached in one call
//! per phase ([`Logger::tee`] for the token stream, [`Logger::log_tree`] /
//! [`Logger::log_syntax_error`] for parse output) — the driver pipes data through and
//! decides control flow, nothing more.

use std::collections::HashSet;
use std::path::Path;

use etac_errors::{Diag, DiagCtxt, ErrorGuaranteed, etac_error};
use etac_parse::{IParser, Parsed};
use etac_session::{cli::Flags, logger::Logger};
use etac_span::{FileId, InterfaceId, SourceCache, SourceId, Span};

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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum File {
    Program(SourceId),
    Interface(InterfaceId),
}

fn parse_path(dcx: &DiagCtxt<'_>, file: &Path) -> Result<File> {
    let Some(file_str) = file.to_str() else {
        return Err(dcx.err_no_span(format!("non-UTF8 file name {}", file.to_string_lossy())).emit());
    };

    match file.extension().and_then(|x| x.to_str()) {
        Some("eta") => Ok(File::Program(SourceId::new(file_str))),
        Some("eti") => Ok(File::Interface(InterfaceId::new(file_str))),
        ext => Err(dcx.err_no_span(format!("unknown file type {}", ext.unwrap_or(""))).emit()),
    }
}

fn find_use_interface(me: SourceId, sym: &str) -> InterfaceId {
    let dir = Path::new(me.as_str())
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let path = dir.join(format!("{sym}.eti"));
    InterfaceId::new(path.to_string_lossy())
}

fn load_file<'src>(cache: &'src SourceCache, dcx: &DiagCtxt<'src>, file: FileId, blame: LoadBlame) -> Result<(u32, &'src str)> {
    match cache.load(file) {
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



fn parse_one<'dcx, 'src, P>(
    logger: &'dcx Logger,
    cache: &'src SourceCache,
    dcx: &'dcx DiagCtxt<'src>,
    file: FileId,
    blame: LoadBlame,
    parser: P,
) -> Result<Option<P::Out>>
where
    P: IParser<'dcx, 'src>,
    P::Out: std::fmt::Display,
    'src: 'dcx,
{
    let (base, source) = load_file(cache, dcx, file, blame)?;

    let lexer = etac_lexer::Lexer::new(base, source, dcx);

    let mut lexer = match (logger.lex, blame) {
        (true, LoadBlame::CommandLine) => compat::ULexer::Tee(logger.tee_lexer(file, cache, lexer)),
        _ => compat::ULexer::Raw(lexer),
    };

    let mut parser = match (logger.parse, blame) {
        (true, LoadBlame::CommandLine) => compat::UParser::Tee(logger.tee_parser(file, cache, parser)),
        _ => compat::UParser::Raw(parser),
    };

    let out = match parser.parse(&mut lexer) {
        Parsed::Ok(tree) | Parsed::Recovered(tree) => Some(tree),
        Parsed::Failed => {
            lexer.for_each(|i| {let _ = i.map_err(Diag::emit);});
            None
        }
    };

    parser
        .into_errors()
        .into_iter()
        .for_each(|e| {e.emit();});

    Ok(out)
}

/// Runs the compiler with the given flags. Errors are emitted as side effects, returns a result
/// that indicates whether or not the program was able to compile
/// # Errors 
/// when the program is not able to be compiled 
pub fn run(flags: &Flags) -> CompilationResult {
    let cache = SourceCache::new();
    let logger = Logger::new(flags);
    let dcx = DiagCtxt::new(&cache);

    let mut files: HashSet<File> = HashSet::new();

    // decode paths for all the files passed in `flags`
    // exits with `Err` on a failure to parse path but doesn't check existance
    for file in &flags.source_files {
        let id = parse_path(&dcx, file.as_path())
            .map_err(|_| CompilationFailure::from(&dcx))?;
        files.insert(id);
    }

    let mut programs = Vec::new();
    let mut interfaces = Vec::new();

    for file in files {
        match file {
            File::Program(p) => {
                let pparser = etac_parse::ProgramParser::new(&dcx);
                let parsed = parse_one(&logger, &cache, &dcx, p, LoadBlame::CommandLine, pparser)
                    .map_err(|_| CompilationFailure::from(&dcx))?;
                let Some(program) = parsed else { continue };
                // uses
                for u in &program.uses {
                    let i = find_use_interface(p, &u.id.sym);
                    let iparser = etac_parse::InterfaceParser::new(&dcx);
                    let parsed = parse_one(&logger, &cache, &dcx, i, LoadBlame::Use(u.span), iparser);
                    let Ok(Some(interface)) = parsed else { continue };
                    interfaces.push(interface);
                }
                programs.push(program);
            },
            File::Interface(i) => {
                let parser = etac_parse::InterfaceParser::new(&dcx);
                let parsed = parse_one(&logger, &cache, &dcx, i, LoadBlame::CommandLine, parser);
                let Ok(Some(interface)) = parsed else { continue };
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
