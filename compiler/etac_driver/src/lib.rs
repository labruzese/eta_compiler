//! The driver for the compiler
//!
//! Is responsible for passing input between each phase and attaching the
//! --lex, --parse, etc. loggers to each phase.

use etac_errors::{Diagnostic, Level, emit, error};
use etac_lexer::Token;
use etac_parse::ParseResult;
use etac_session::{cli::Flags, logger::Logger};
use etac_span::{FileId, InterfaceId, SourceCache, SourceId};

/// Runs the compiler with the given flags. Errors are emitted as side effects, returns a result
/// that indicates whether or not the program was able to compile
pub fn run(flags: Flags) -> Result<(), ()> {
    let mut cache = SourceCache::new();
    let mut sources: Vec<SourceId> = vec![];
    let mut interfaces: Vec<InterfaceId> = vec![];

    // decode paths for all the files passed in `flags`
    // exits with `Err` on a failure to parse path but doesn't check existance
    for file in &flags.source_files {
        let Some(file_str) = file.to_str() else {
            emit(&mut cache, error!("non-UTF8 file name {}", file.to_string_lossy()));
            return Err(());
        };
        match file.extension().and_then(|x| x.to_str()) {
            Some("eta") => sources.push(SourceId::new(file_str)),
            Some("eti") => interfaces.push(InterfaceId::new(file_str)),
            ext => emit(&mut cache, error!("unknown file type {}", ext.unwrap_or(""))),
        }
    }

    let logger = Logger::new(&flags);

    let _interfaces: Vec<_> = interfaces
        .iter()
        .map(|interface| drive_parser::<_, etac_parse::InterfaceParser>(&flags, &mut cache, &logger, interface))
        .collect::<Result<_, _>>()?;

    let _programs: Vec<_> = sources
        .iter()
        .map(|program| drive_parser::<_, etac_parse::ProgramParser>(&flags, &mut cache, &logger, program))
        .collect::<Result<_, _>>()?;

    Ok(())
}

/// Helper to drive a specific a parser. 
fn drive_parser<Out, Parser>(
    flags: &Flags,
    cache: &mut SourceCache,
    logger: &Logger,
    file_id: &FileId,
) -> Result<Out, ()>
where
    Parser: etac_parse::IParser<Out>,
    Out: std::fmt::Display,
{
    // place the file into our cache (global source file), gets back the offset and the read
    // source from disk, fails if file doesn't exist
    let (base, source) = match cache.load(file_id) {
        Ok(s) => s,
        Err(io_err) => {
            emit(cache, io_err.into());
            return Err(());
        }
    };

    let mut lex_logging = flags.lex;
    let parse_logging = flags.parse;

    // callback closure for our lexer, is a no-op if lex_logging is disabled
    let tok_map_fn = |lex_result| {
        if lex_logging {
            token_callback(logger, file_id, &cache, &lex_result)?;
            if lex_result.is_err() {
                lex_logging = false
            }
        }
        lex_result
    };

    // lexer with attached (side-effect only) callback
    let mut lexer = etac_lexer::Lexer::new(base, &source).map(tok_map_fn);

    // parse logging callbacks happen here
    match etac_parse::parse::<_, _, Parser>(&mut lexer) {
        ParseResult::Clean(out) => {
            if parse_logging {
                parse_clean_cb(logger, cache, file_id, &out)?;
            }
            Ok(out)
        }
        ParseResult::WithDiags { out, diags } => {
            if parse_logging {
                parse_with_diags_cb(logger, cache, file_id, &diags)?;
            }
            for d in diags {
                emit(cache, d);
            }
            Ok(out)
        }
        ParseResult::FatalError(diags) => {
            // drain lexer for more diagnostics
            lexer.for_each(drop);
            if parse_logging {
                parse_fatal_cb(logger, cache, file_id, &diags)?;
            }
            for d in dbg!(diags) {
                emit(cache, d)
            }
            Err(())
        }
    }
}

// --- callbacks ---

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
            if diag.level == etac_errors::Level::Error {
                logger.log_lexical_error(file_id, loc, &diag.message)?
            };
        }
    }
    Ok(())
}

fn parse_clean_cb<Out: std::fmt::Display>(
    logger: &Logger,
    cache: &mut SourceCache,
    file_id: &FileId,
    out: &Out,
) -> Result<(), ()> {
    logger.log_parse(&file_id, out).map_err(|e| emit(cache, e.into()))?;
    Ok(())
}

fn parse_with_diags_cb(
    logger: &Logger,
    cache: &mut SourceCache,
    file_id: &FileId,
    diags: &Vec<Diagnostic>,
) -> Result<(), ()> {
    if let Some(error) = diags.iter().find(|d| d.level == Level::Error) {
        logger
            .log_syntactic_error(
                &file_id,
                cache
                    .lc_index(error.loc.expect("syntactic error has location").lo)
                    .map_err(|e| emit(cache, e.into()))?,
                &error.message,
            )
            .map_err(|e| emit(cache, e.into()))?;
    };
    Ok(())
}

fn parse_fatal_cb(
    logger: &Logger,
    cache: &mut SourceCache,
    file_id: &FileId,
    diags: &Vec<Diagnostic>,
) -> Result<(), ()> {
    let diag = diags.iter().find(|d| d.level == Level::Error).unwrap();
    let loc = cache
        .lc_index(
            diag.loc.as_ref().expect("syntactic error must have location").lo,
        )
        .map_err(|e| emit(cache, e.into()))?;
    logger
        .log_syntactic_error(&file_id, loc, &diag.message)
        .map_err(|e| emit(cache, e.into()))?;
    Ok(())
}
