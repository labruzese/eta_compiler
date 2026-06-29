//! The driver for the compiler
//!
//! Is responsible for passing input between each phase and attaching the
//! --lex, --parse, etc. loggers to each phase.
//!
//! Every diagnostic in the pipeline flows through a single [`DiagCtxt`] created here.
//! The driver no longer collects `Vec<Diagnostic>` to drain later: each phase emits as
//! it goes, and the driver only decides control flow from the phase's result.

use etac_errors::{DiagCtxt, Diagnostic, Level};
use etac_lexer::Token;
use etac_parse::Parsed;
use etac_session::{cli::Flags, logger::Logger};
use etac_span::{FileId, InterfaceId, SourceCache, SourceId};

/// Runs the compiler with the given flags. Errors are emitted as side effects, returns a result
/// that indicates whether or not the program was able to compile
pub fn run(flags: Flags) -> Result<(), ()> {
    let cache = SourceCache::new();
    // The one and only diagnostic context. Borrows `cache` (interior-mutable) so it can
    // render spans; every phase below reports through it.
    let dcx = DiagCtxt::new(&cache);

    let mut sources: Vec<SourceId> = vec![];
    let mut interfaces: Vec<InterfaceId> = vec![];

    // decode paths for all the files passed in `flags`
    // exits with `Err` on a failure to parse path but doesn't check existance
    for file in &flags.source_files {
        let Some(file_str) = file.to_str() else {
            dcx.err_no_span(format!("non-UTF8 file name {}", file.to_string_lossy()))
                .emit();
            return Err(());
        };
        match file.extension().and_then(|x| x.to_str()) {
            Some("eta") => sources.push(SourceId::new(file_str)),
            Some("eti") => interfaces.push(InterfaceId::new(file_str)),
            ext => {
                dcx.err_no_span(format!("unknown file type {}", ext.unwrap_or("")))
                    .emit();
            }
        }
    }

    let logger = Logger::new(&flags);

    let _programs: Vec<_> = sources
        .iter()
        .map(|program| {
            drive_parser::<_, etac_parse::ProgramParser>(&flags, &dcx, &logger, program).inspect(
                |etac_ast::Program { uses, definitions: _, .. }| {
                    for u in uses {
                        interfaces.push(InterfaceId::new(u.id.sym.as_str()))
                    }
                },
            )
        })
        .collect::<Result<_, _>>()?;

    let _interfaces: Vec<_> = interfaces
        .iter()
        .map(|interface| drive_parser::<_, etac_parse::InterfaceParser>(&flags, &dcx, &logger, interface))
        .collect::<Result<_, _>>()?;

    Ok(())
}

/// Helper to drive a specific a parser.
fn drive_parser<Out, Parser>(
    flags: &Flags,
    dcx: &DiagCtxt,
    logger: &Logger,
    file_id: &FileId,
) -> Result<Out, ()>
where
    Parser: etac_parse::IParser<Out>,
    Out: std::fmt::Display,
{
    let cache = dcx.sources();

    // place the file into our cache (global source file), gets back the offset and the read
    // source from disk, fails if file doesn't exist
    let (base, source) = match cache.load(file_id) {
        Ok(s) => s,
        Err(io_err) => {
            dcx.emit(io_err.into());
            return Err(());
        }
    };

    let mut lex_logging = flags.lex;
    let parse_logging = flags.parse;

    // callback closure for our lexer, is a no-op if lex_logging is disabled
    let tok_map_fn = |lex_result| {
        if lex_logging {
            token_callback(logger, file_id, cache, &lex_result)?;
            if lex_result.is_err() {
                lex_logging = false
            }
        }
        lex_result
    };

    // lexer with attached (side-effect only) callback
    let mut lexer = etac_lexer::Lexer::new(base, &source).map(tok_map_fn);

    // The parser emits every diagnostic through `dcx` itself; here we only log and pick
    // a control-flow path from the outcome.
    match etac_parse::parse::<_, _, Parser>(dcx, &mut lexer) {
        Parsed::Ok(out) => {
            if parse_logging {
                log_parse_tree(logger, dcx, file_id, &out)?;
            }
            Ok(out)
        }
        Parsed::Recovered { out, first_error, .. } => {
            if parse_logging {
                log_first_syntax_error(logger, dcx, file_id, &first_error)?;
            }
            Ok(out)
        }
        Parsed::Failed { first_error, .. } => {
            // drain lexer so the `.lexed` log still captures every token (and the first
            // lexical error) even though parsing stopped early
            lexer.for_each(drop);
            if parse_logging {
                log_first_syntax_error(logger, dcx, file_id, &first_error)?;
            }
            Err(())
        }
    }
}

// --- logging glue ---
//
// These only write the external `.lexed`/`.parsed` logs. They never emit diagnostics for
// the *compiled program* — the parser already did that through `dcx`. They do route their
// own I/O failures into `dcx` so a broken log file surfaces like any other error.

fn token_callback(
    logger: &Logger,
    file_id: &FileId,
    cache: &SourceCache,
    lex_result: &Result<(usize, Token, usize), Diagnostic>,
) -> Result<(), Diagnostic> {
    match lex_result {
        Ok((start, tok, _end)) => {
            let loc = cache.lc_index(*start)?;
            logger.log_token(file_id, loc, tok)?;
        }
        Err(diag) => {
            let loc = cache.lc_index(diag.loc.as_ref().expect("lexcial error must have location").lo)?;
            if diag.level == Level::Error {
                logger.log_lexical_error(file_id, loc, &diag.message)?
            };
        }
    }
    Ok(())
}

fn log_parse_tree<Out: std::fmt::Display>(
    logger: &Logger,
    dcx: &DiagCtxt,
    file_id: &FileId,
    out: &Out,
) -> Result<(), ()> {
    logger.log_parse(file_id, out).map_err(|e| {
        dcx.emit(e.into());
    })
}

fn log_first_syntax_error(
    logger: &Logger,
    dcx: &DiagCtxt,
    file_id: &FileId,
    first_error: &Diagnostic,
) -> Result<(), ()> {
    let cache = dcx.sources();
    let loc = first_error.loc.as_ref().expect("syntactic error must have location");
    let at = cache.lc_index(loc.lo).map_err(|e| {
        dcx.emit(e.into());
    })?;
    logger.log_syntactic_error(file_id, at, &first_error.message).map_err(|e| {
        dcx.emit(e.into());
    })
}
