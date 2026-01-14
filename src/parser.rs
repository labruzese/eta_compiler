mod lexer_logger;
use crate::cli;
use crate::lexer;
use crate::sources::{EtaSpan, FileId, SourceManager};
use lexer_logger::*;

// Update signature to take file_id and the manager
pub fn parse(sm: &SourceManager, file_id: &FileId) {
    let options = cli::flags();
    let sname = sm.get_file_name(file_id).expect("err getting file name");
    let source = sm
        .get_source(file_id)
        .expect("unknown source encountered in parser");
    let lexer = lexer::Lexer::new(&source).spanned();

    // make some writer for verbose lexing if flag is set
    let mut logger = LexerLogger::new(options, sname, source.clone());

    // iterate tokens
    for (token_result, span) in lexer {
        match token_result {
            // Successful token
            Ok(token) => {
                // if we have a writer write our lexing output
                logger.log_token(span.start, &token);

                // TODO: actual parsing logic
            }
            // diagnostic produced by lexer
            Err(diag) => {
                let eta_span: EtaSpan = (file_id, span).into();
                //emit
                logger.log_error(eta_span.range.start, &diag.message);
                sm.emit(diag.specify_file(file_id), eta_span);
                break; // fail on lex error
            }
        }
    }

    logger.flush();
}
