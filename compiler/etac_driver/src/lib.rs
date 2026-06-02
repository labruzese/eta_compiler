use etac_errors::{emit, error, Diagnostic, Level};
use etac_lexer::Token;
use etac_parse::ParseResult;
use etac_session::{cli::Flags, logger::Logger};
use etac_span::{FileId, InterfaceId, SourceId, SourceCache};

pub fn run(flags: Flags) -> Result<(), ()> {
    let mut cache = SourceCache::new();
    let mut sources: Vec<SourceId> = vec![];
    let mut interfaces: Vec<InterfaceId> = vec![];

    for file in &flags.source_files {
        let Some(file_str) = file.to_str() else {
            emit(&mut cache, error!("non-UTF9 file name {}", file.to_string_lossy()));
            return Err(());
        };
        match file.extension().and_then(|x| x.to_str()) {
            Some("eta") => sources.push(SourceId::new(file_str)),
            Some("eti") => interfaces.push(InterfaceId::new(file_str)),
            ext => emit(&mut cache, error!("unknown file type {}", ext.unwrap_or(""))),
        }
    }
    let logger = Logger::new(&flags);
    drive::<_, etac_parse::InterfaceParser>(&flags, &mut cache, &logger, &interfaces)?;
    drive::<_, etac_parse::ProgramParser>(&flags, &mut cache, &logger, &sources)?;
    Ok(())
}

fn drive<Out, Parser>(flags: &Flags, cache: &mut SourceCache, logger: &Logger, files: &[FileId]) -> Result<(), ()>
where
    Parser: etac_parse::IParser<Out>,
    Out: std::fmt::Display,
{
    for file_id in files {
        let (base, source) = match cache.load(file_id) {
            Ok(s) => s,
            Err(io_err) => {
                emit(cache, io_err.into());
                return Err(());
            }
        };

        let mut lex_logging = flags.lex;
        let parse_logging = flags.parse;

        let tok_map_fn = |lex_result| {
            if lex_logging {
                token_callback(logger, file_id, &cache, &lex_result)?;
                if lex_result.is_err() {
                    lex_logging = false
                }
            }
            lex_result
        };

        let mut lexer = etac_lexer::Lexer::new(base, &source).map(tok_map_fn);
        let parse_res = etac_parse::parse::<_, _, Parser, _>(&mut lexer, &mut |x| x);

        match parse_res {
            ParseResult {
                output: Some(out),
                errors: empty,
            } if empty.is_empty() => {
                if parse_logging {
                    logger.log_parse(&file_id, out).map_err(|e| emit(cache, e.into()))?;
                };
                eprintln!("successfully parsed :)");
            }
            ParseResult {
                output: Some(_out),
                errors: diags,
            } if diags.iter().any(|d| d.level == Level::Error) => {
                if parse_logging {
                    let diag = diags.iter().find(|d| d.level == Level::Error).unwrap();
                    let loc = cache
                        .lc_index(
                            file_id,
                            diag.loc
                                .as_ref()
                                .expect("syntactic error must have location")
                                .lo,
                        )
                        .map_err(|e| emit(cache, e.into()))?;
                    logger
                        .log_syntactic_error(&file_id, loc, &diag.message)
                        .map_err(|e| emit(cache, e.into()))?;
                }
                for d in diags {
                    emit(cache, d);
                }
                eprintln!("parse tree can still be typechecked");
            }
            ParseResult {
                output: None,
                errors: diags,
            } => {
                debug_assert!(!diags.is_empty());
                // drain lexer for more diagnostics
                lexer.for_each(drop);
                if parse_logging {
                    let diag = diags.iter().find(|d| d.level == Level::Error).unwrap();
                    let loc = cache
                        .lc_index(
                            file_id,
                            diag.loc
                                .as_ref()
                                .expect("syntactic error must have location")
                                .lo,
                        )
                        .map_err(|e| emit(cache, e.into()))?;
                    logger
                        .log_syntactic_error(&file_id, loc, &diag.message)
                        .map_err(|e| emit(cache, e.into()))?;
                }
                for d in dbg!(diags) {
                    emit(cache, d)
                }
                eprintln!("couldn't build parse tree");
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}

fn token_callback(
    logger: &Logger,
    file_id: &FileId,
    cache: &SourceCache,
    lex_result: &Result<(usize, Token, usize), Diagnostic>,
) -> Result<(), Diagnostic> {
    match lex_result {
        Ok((start, tok, _end)) => {
            let loc = cache.lc_index(file_id, *start)?;
            logger.log_token(file_id, loc, tok)?;
        }
        Err(diag) => {
            let loc = cache.lc_index(
                file_id,
                diag.loc.as_ref().expect("lexcial error must have location").lo,
            )?;
            if diag.level == etac_errors::Level::Error {
                logger.log_lexical_error(file_id, loc, &diag.message)?
            };
        }
    }
    Ok(())
}
