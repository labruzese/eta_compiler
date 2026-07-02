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

use etac_errors::{Diag, DiagCtxt};
use etac_parse::{IParser, Parsed};
use etac_session::{cli::Flags, logger::Logger};
use etac_span::{InterfaceId, SourceCache, SourceId};

#[derive(Debug)]
pub struct CompilationFailure {
    pub errors: usize,
    pub warnings: usize,
}
impl From<&DiagCtxt<'_>> for CompilationFailure {
    fn from(value: &DiagCtxt<'_>) -> Self {
        CompilationFailure {
            errors: value.err_count(),
            warnings: value.warn_count(),
        }
    }
}
pub struct CompilationSuccess {
    pub warnings: usize
}
impl From<&DiagCtxt<'_>> for CompilationSuccess {
    fn from(value: &DiagCtxt<'_>) -> Self {
        CompilationSuccess {
            warnings: value.warn_count(),
        }
    }
}

/// Runs the compiler with the given flags. Errors are emitted as side effects, returns a result
/// that indicates whether or not the program was able to compile
/// # Errors 
/// when the program is not able to be compiled 
pub fn run(flags: &Flags) -> Result<CompilationSuccess, CompilationFailure> {
    let cache = SourceCache::new();

    // Captures the --lex/--parse flags; phases attach to it by name below.
    let logger = Logger::new(flags);

    // The one and only diagnostic context. Borrows `cache` (interior-mutable) so it can
    // render spans; every phase below reports through it.
    let dcx = DiagCtxt::new(&cache);

    let mut pids: Vec<SourceId> = vec![];
    let mut iids: Vec<InterfaceId> = vec![];

    // decode paths for all the files passed in `flags`
    // exits with `Err` on a failure to parse path but doesn't check existance
    for file in &flags.source_files {
        let Some(file_str) = file.to_str() else {
            dcx.err_no_span(format!("non-UTF8 file name {}", file.to_string_lossy()))
                .emit();
            return Err((&dcx).into());
        };
        match file.extension().and_then(|x| x.to_str()) {
            Some("eta") => pids.push(SourceId::new(file_str)),
            Some("eti") => iids.push(InterfaceId::new(file_str)),
            ext => {
                dcx.err_no_span(format!("unknown file type {}", ext.unwrap_or("")))
                    .emit();
            }
        }
    }


    let mut programs = Vec::new();
    for program_id in pids {
        // make parser
        let parser = etac_parse::ProgramParser::new(&dcx);
        let mut parser = logger.tee_parser(program_id, &cache, parser);
        // load source
        let (base, source) = cache.load(program_id).map_err(|ioe| { Diag::io(&dcx, &ioe).emit(); CompilationFailure::from(&dcx)})?;
        // make lexer
        let lexer = etac_lexer::Lexer::new(base, source, &dcx);
        let mut lexer = logger.tee_lexer(program_id, &cache, lexer);
        // parse
        match parser.parse(&mut lexer) {
            Parsed::Ok(program) |
            Parsed::Recovered(program) => {
                for u in &program.uses {
                    iids.push(InterfaceId::new(u.id.sym.as_str()));
                }
                programs.push(program);
            },
            Parsed::Failed => {
                // produce extra diagnostics
                let _ = lexer.map(|t| t.map_err(etac_errors::Diag::emit));
            },
        }
        for err in parser.into_errors() {
            let _guar = err.emit();
        }
    }

    let mut interfaces = Vec::new();
    for interface_id in iids {
        // make parser
        let parser = etac_parse::InterfaceParser::new(&dcx);
        let mut parser = logger.tee_parser(interface_id, &cache, parser);
        // load source
        let (base, source) = cache.load(interface_id).map_err(|ioe| { Diag::io(&dcx, &ioe).emit(); CompilationFailure::from(&dcx)})?;
        // make lexer
        let lexer = etac_lexer::Lexer::new(base, source, &dcx);
        let mut lexer = logger.tee_lexer(interface_id, &cache, lexer);
        // parse
        match parser.parse(&mut lexer) {
            Parsed::Ok(interface) |
            Parsed::Recovered(interface) => {
                interfaces.push(interface);
            },
            Parsed::Failed => {
                // produce extra diagnostics
                let _ = lexer.map(|t| t.map_err(etac_errors::Diag::emit));
            },
        }

        for err in parser.into_errors() {
            let _guar = err.emit();
        }
    }

    match dcx.has_errors() {
        Some(_)=> Err((&dcx).into()),
        None => Ok((&dcx).into()),
    }
}
