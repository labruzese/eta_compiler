use std::fs::OpenOptions;
use std::io::{BufWriter, Write};

use crate::flags;
use crate::lexer;
use crate::sources::{EtaSpan, FileId, SourceManager};

#[derive(Debug)]
pub enum ParseError {
    IOError(std::io::Error),
    UnknownSource,
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::IOError(err)
    }
}

// Update signature to take file_id and the manager
pub fn parse(sm: SourceManager, file_id: FileId) {
    let options = flags::flags();
    let source = sm
        .get_source(&file_id)
        .expect("unknown source encountered in parser");
    let lexer = lexer::Lexer::new(&source).spanned();

    // make some writer for verbose lexing if flag is set
    let (mut writer, location_resolver) = if options.lex {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&options.lex_file)
            .expect("unable to open lex file to write in parser");

        (
            Some(BufWriter::new(file)),
            // this is slower than nessacary, we should switch to tracking location inside the
            // lexer
            Some(ariadne::Source::from(&source)),
        )
    } else {
        (None, None)
    };

    // iterate tokens
    for (token_result, span) in lexer {
        match token_result {
            // Successful token
            Ok(token) => {
                // if we have a writer write our lexing output
                if let Some(w) = &mut writer
                    && let Some(lresolver) = &location_resolver
                {
                    //these are 0 indexed
                    let (_, line, col) = lresolver
                        .get_byte_line(span.start)
                        .expect("couldn't resolve location from byte offset");
                    writeln!(w, "{}:{} {}", line + 1, col + 1, token)
                        .expect("failed to write to lex file buffer");
                }

                // TODO: actual parsing logic
            }
            // diagnostic produced by lexer
            Err(diag) => {
                let eta_span: EtaSpan = (&file_id, span).into();
                // We modify the existing Diagnostic to add the label
                // Since this comes from the Lexer (e.g., bad int), we flag the specific text.

                // emit to lexer if option set
                if let Some(w) = &mut writer
                    && let Some(lresolver) = &location_resolver
                {
                    let (_, line, col) = lresolver
                        .get_byte_line(eta_span.range.start)
                        .expect("couldn't resolve location from byte offset");
                    writeln!(w, "{}:{} error:{}", line + 1, col + 1, diag.message)
                        .expect("failed to write to lex file buffer");
                }

                let binded_diag = diag.specify_file(&file_id);

                // Emit the error
                sm.emit(binded_diag, eta_span);
            }
        }
    }

    if let Some(mut w) = writer {
        w.flush().expect("failed to flush writer to lex file");
    }
}
