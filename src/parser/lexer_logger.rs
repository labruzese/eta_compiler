use crate::cli;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::rc::Rc;

/// Handles printing the lexer log if enabled
pub struct LexerLogger {
    writer: Option<BufWriter<std::fs::File>>,
    // this is slower than nessacary, we should switch to tracking location inside the
    // lexer
    resolver: Option<ariadne::Source<Rc<str>>>,
}

impl LexerLogger {
    pub fn new(options: &cli::Flags, name: &str, source: Rc<str>) -> Self {
        if options.lex {
            let mut path = options.diag_path.clone();
            path.push(format!("{}.lexed", name));
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .expect("unable to open lex file to write in parser");

            Self {
                writer: Some(BufWriter::new(file)),
                resolver: Some(ariadne::Source::from(source)),
            }
        } else {
            Self {
                writer: None,
                resolver: None,
            }
        }
    }

    pub fn log_token(&mut self, byte_offset: usize, token: &impl std::fmt::Display) {
        if let Some(w) = &mut self.writer
            && let Some(lresolver) = &self.resolver
        {
            //these are 0 indexed
            let (_, line, col) = lresolver
                .get_byte_line(byte_offset)
                .expect("couldn't resolve location from byte offset");
            writeln!(w, "{}:{} {}", line + 1, col + 1, token)
                .expect("failed to write to lex file buffer");
        }
    }

    pub fn log_error(&mut self, byte_offset: usize, message: &str) {
        if let Some(w) = &mut self.writer
            && let Some(lresolver) = &self.resolver
        {
            let (_, line, col) = lresolver
                .get_byte_line(byte_offset)
                .expect("couldn't resolve location from byte offset");
            writeln!(w, "{}:{} error:{}", line + 1, col + 1, message)
                .expect("failed to write to lex file buffer");
        }
    }

    pub fn flush(&mut self) {
        if let Some(w) = &mut self.writer {
            w.flush().expect("failed to flush writer to lex file");
        }
    }
}
