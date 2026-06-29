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

use etac_errors::DiagCtxt;
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

    // Captures the --lex/--parse flags; phases attach to it by name below.
    let logger = Logger::new(&flags);

    let _programs: Vec<_> = sources
        .iter()
        .map(|program| {
            drive_parser::<_, etac_parse::ProgramParser>(&dcx, &logger, program).inspect(
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
        .map(|interface| drive_parser::<_, etac_parse::InterfaceParser>(&dcx, &logger, interface))
        .collect::<Result<_, _>>()?;

    Ok(())
}

/// Helper to drive a specific a parser.
fn drive_parser<Out, Parser>(dcx: &DiagCtxt, logger: &Logger, file_id: &FileId) -> Result<Out, ()>
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

    // Attach --lex logging in one line: a transparent pass-through unless --lex is set.
    let mut lexer = logger.tee(file_id, cache, etac_lexer::Lexer::new(base, &source));

    // The parser emits every diagnostic through `dcx` itself; here we only log the result
    // and pick a control-flow path from the outcome.
    match etac_parse::parse::<_, _, Parser>(dcx, &mut lexer) {
        Parsed::Ok(out) => {
            logger.log_tree(file_id, &out);
            Ok(out)
        }
        Parsed::Recovered { out, first_error, .. } => {
            logger.log_syntax_error(file_id, cache, &first_error);
            Ok(out)
        }
        Parsed::Failed { first_error, .. } => {
            // drain the lexer so the `.lexed` log still captures every token (and the
            // first lexical error) even though parsing stopped early
            lexer.for_each(drop);
            logger.log_syntax_error(file_id, cache, &first_error);
            Err(())
        }
    }
}
