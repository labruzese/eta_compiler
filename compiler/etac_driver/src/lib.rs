use etac_lexer::Token;
use etac_session::{cli::Flags, logger::Logger};
use etac_span::{FileId, InterfaceId, SourceId, Sources};
use etac_errors::{emit, error, Diagnostic};

pub fn run(flags: Flags) -> Result<(), ()> {
    let mut cache = Sources::new();
    let mut sources: Vec<SourceId> = vec![];
    let mut interfaces: Vec<InterfaceId> = vec![];

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
    drive::<_, etac_parse::InterfaceParser>(&flags, &mut cache, &logger, &interfaces)?;
    drive::<_, etac_parse::ProgramParser>(&flags, &mut cache, &logger, &sources)?;
    Ok(())
}

fn drive<Out, Parser>(
    flags: &Flags,
    cache: &mut Sources,
    logger: &Logger,
    files: &[FileId],
) -> Result<(),()> 
where
    Parser: etac_parse::IParser<Out>,
    Out: std::fmt::Display,
{
    for file_id in files {
        let source = match cache.text(file_id) {
            Ok(s) => s,
            Err(io_err) => {
                emit(cache, io_err.into());
                return Err(());
            }
        };
        
        let mut lex_logging = flags.lex;
        let mut parse_logging = flags.parse;

        let tok_map_fn = |lex_result| {
            if lex_logging {
                token_callback(logger, file_id, &cache, &lex_result)?;
                if lex_result.is_err() { lex_logging = false }
            }
            lex_result
        };
        let mut parse_cb_fn = |parse_result: Result<Out, Diagnostic>| {
            if parse_logging {
                parse_callback(logger, file_id, &cache, &parse_result)?;
                if parse_result.is_err() { parse_logging = false }
            }
            parse_result
        };

        let mut lexer = etac_lexer::Lexer::new(file_id.clone(), &source).map(tok_map_fn);
        let parse_res = etac_parse::parse::<_, _, Parser, _>(file_id, &mut lexer, &mut parse_cb_fn);

        if let Err(diags) = parse_res {
            // drain lexer when parse crashes
            lexer.for_each(drop);
            for d in diags {
                emit(cache, d);
            }
            return Err(());
        }
    }
    Ok(())
}

fn token_callback(logger: &Logger, file_id: &FileId, cache: &Sources, lex_result: &Result<(usize, Token, usize), Diagnostic>) -> Result<(), Diagnostic> {
    match lex_result {
        Ok((start, tok, _end)) => {
            let loc = cache.lc_index(file_id, *start)?;
            logger.log_token(file_id, loc, tok)?;
        },
        Err(diag) => {
            let loc = cache.lc_index(file_id, diag.loc.as_ref().expect("lexcial error must have location").range.start)?;
            logger.log_lexical_error(file_id, loc, &diag.message)?;
        },
    }
    Ok(())
}

fn parse_callback<O: std::fmt::Display>(logger: &Logger, file_id: &FileId, cache: &Sources, parse_result: &Result<O, Diagnostic>) -> Result<(), Diagnostic> {
    match parse_result {
        Ok(out) => logger.log_parse(file_id, out)?,
        Err(diag) => {
            let loc = cache.lc_index(file_id, diag.loc.as_ref().expect("syntactic error must have location").range.start)?;
            logger.log_syntactic_error(file_id, loc, &diag.message)?;
        },
    }
    Ok(())
}
