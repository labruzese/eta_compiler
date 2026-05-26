use etac_lexer::Token;
use etac_session::{cli::Flags, logger::Logger};
use etac_span::{FileId, InterfaceId, SourceId, Sources};
use etac_errors::{emit, error, Diagnostic};
use std::{cell::{RefCell}};

pub fn run(flags: Flags) -> Result<(), ()> {
    let cache = RefCell::new(Sources::new());
    let mut sources: Vec<SourceId> = vec![];
    let mut interfaces: Vec<InterfaceId> = vec![];

    for file in &flags.source_files {
        let Some(file_str) = file.to_str() else {
            emit(&mut cache.borrow_mut(), error!("non-UTF8 file name {}", file.to_string_lossy()));
            return Err(());
        };
        match file.extension().and_then(|x| x.to_str()) {
            Some("eta") => sources.push(SourceId::new(file_str)),
            Some("eti") => interfaces.push(InterfaceId::new(file_str)),
            ext => emit(&mut cache.borrow_mut(), error!("unknown file type {}", ext.unwrap_or(""))),
        }
    }
    let logger = Logger::new(&flags);
    drive::<_, etac_parse::InterfaceParser>(&cache, &logger, &interfaces)?;
    drive::<_, etac_parse::ProgramParser>(&cache, &logger, &sources)?;
    Ok(())
}

fn drive<Out, Parser>(
    cache: &RefCell<Sources>,
    logger: &Logger,
    files: &[FileId],
) -> Result<(),()> 
where
    Parser: etac_parse::IParser<Out>,
    Out: std::fmt::Display,
{
    for file_id in files {
        let source = match cache.borrow_mut().text(file_id) {
            Ok(s) => s,
            Err(io_err) => {
                emit(&mut cache.borrow_mut(), error!(file_id.clone(); "io error: {}", io_err));
                return Err(());
            }
        };

        let tok_map_fn = |lex_result| {
            token_callback(logger, file_id, &mut cache.borrow_mut(), &lex_result);
            lex_result
        };
        let mut parse_cb_fn = |parse_result: &Result<Out, Diagnostic>| {
            parse_callback(logger, file_id, &mut cache.borrow_mut(), parse_result)
        };

        let mut lexer = etac_lexer::Lexer::new(file_id.clone(), &source).map(tok_map_fn);
        let parse_res = etac_parse::parse::<_, _, Parser, _>(file_id, &mut lexer, &mut parse_cb_fn);

        if let Err(diags) = parse_res {
            for d in diags {
                emit(&mut cache.borrow_mut(), d);
            }
            // drain lexer if we have --lex
            if logger.lex_enabled() {
                let _ = lexer.for_each(drop);
            }
            return Err(());
        }
    }
    Ok(())
}

fn token_callback(logger: &Logger, file_id: &FileId, cache: &mut Sources, lex_result: &Result<(usize, Token, usize), Diagnostic>) {
    match lex_result {
        Ok((start, tok, _end)) => {
            let loc = cache.lc_index(file_id, *start).expect("io error in token callback");
            logger.log_token(file_id, loc, tok);
        },
        Err(diag) => {
            let loc = cache.lc_index(file_id, diag.loc.as_ref().expect("lexcial error must have location").range.start).expect("io error in token callback");
            logger.log_lexical_error(file_id, loc, &diag.message);
        },
    }
}

fn parse_callback<O: std::fmt::Display>(logger: &Logger, file_id: &FileId, cache: &mut Sources, parse_result: &Result<O, Diagnostic>) {
    match parse_result {
        Ok(out) => logger.log_parse(file_id, out),
        Err(diag) => {
            let loc = cache.lc_index(file_id, diag.loc.as_ref().expect("syntactic error must have location").range.start).expect("io error in token callback");
            logger.log_syntactic_error(file_id, loc, &diag.message);
        },
    }
}
