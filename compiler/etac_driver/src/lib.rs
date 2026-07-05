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

use etac_errors::{Diag, DiagCtxt, ErrorGuaranteed, etac_error};
use etac_parse::{IParser, Parsed};
use etac_session::{cli::Flags, logger::Logger};
use etac_span::{FileId, SourceCache, Span};

pub use crate::resolve::Resolver;
pub use crate::status::{CompilationFailure, CompilationSuccess};

use crate::resolve::File;

mod compat;
mod resolve;
mod status;


type Result<T> = std::result::Result<T, ErrorGuaranteed>;
type CompilationResult = std::result::Result<CompilationSuccess, CompilationFailure>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum LoadBlame {
    CommandLine,
    Use(Span),
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

    // All name-to-file decisions (sourcepath, libpath, dedup) live in the
    // resolver; see the resolve module.
    let mut resolver = Resolver::new(&flags.source_path, &flags.lib_path);

    // Classify the command-line inputs, keeping their order (deterministic
    // diagnostics and logs; the previous HashSet iterated in a random order)
    // and letting the resolver dedup repeats. An unusable path is reported
    // and skipped — it must not silence diagnostics for the other files.
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
                let pparser = etac_parse::ProgramParser::new(&dcx);
                let parsed = parse_one(&logger, &cache, &dcx, p, LoadBlame::CommandLine, pparser)
                    .map_err(|_| CompilationFailure::from(&dcx))?;
                let Some(program) = parsed else { continue };
                // uses (the resolver returns None for already-queued
                // interfaces, so a shared interface is parsed exactly once)
                for u in &program.uses {
                    let Some(i) = resolver.resolve_use(&dcx, p, &u.id.sym, u.span) else {
                        continue;
                    };
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
