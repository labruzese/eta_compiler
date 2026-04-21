use crate::{ast, cli};
use crate::sources::{EtaSpan, FileId, QualifiedName, Sources};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::{PathBuf};
use std::rc::Rc;

/// Handles printing the lexer log if enabled
pub struct Logger {
    lexer_writer: Option<HashMap<FileId, BufWriter<std::fs::File>>>,
    parser_writer: Option<HashMap<FileId, BufWriter<std::fs::File>>>,
    diag_root: PathBuf,
    /// Requirements: Resolver is some iff any writer is some
    resolver: Option<Rc<Sources>>,
}

impl<'source_cache> Logger {
    pub fn new(options: &cli::Flags, sources: Rc<Sources>) -> Self {
        let mut me = Self { 
            lexer_writer:  None,
            parser_writer:  None,
            diag_root: options.diag_path.clone(),
            resolver:  None,
        };
        if options.lex || options.parse {
            let _ = me.resolver.insert(sources);
            std::fs::create_dir_all(&options.diag_path)
                .expect("unable to create diagnostic output directory");
        }
        if options.lex {
            me.lexer_writer = Some(HashMap::new());
        }    
        if options.parse {
            me.parser_writer = Some(HashMap::new());
        }    
        me
    }

    pub fn is_logging_lexer(&self) -> bool { self.lexer_writer.is_some() }
    pub fn is_logging_parser(&self) -> bool { self.parser_writer.is_some() }

    pub fn log_token(&mut self, at: EtaSpan, token: &impl std::fmt::Display) {
        if let Some(w) = &mut self.lexer_writer
        {
            //these are 0 indexed
            let (_, line, col) = self.resolver.unwrap()(at.file_id);
            writeln!(w, "{}:{} {}", line + 1, col + 1, token)
                .expect("failed to write to lex file buffer");
        }
    }

    pub fn log_parse(&mut self, program: &ast::Program) {
        if let Some(w) = &mut self.parser_writer {
            writeln!(w, "{}", program)
                .expect("failed to write to parse file buffer");
        }
    }

    pub fn log_lexical_error(&mut self, byte_start: usize, _byte_end: usize, message: &str) {
        if let Some(w) = &mut self.lexer_writer
        {
            let (_, line, col) = self.resolver
                .as_ref()
                .unwrap()
                .get_byte_line(byte_start)
                .expect("couldn't resolve location from byte offset");
            writeln!(w, "{}:{} error:{}", line + 1, col + 1, message)
                .expect("failed to write to lex file buffer");
            //detach after first report
            self.lexer_writer = None;
        }
    }

    pub fn log_syntatic_error(&mut self, byte_start: usize, _byte_end: usize, message: &str) {
        if let Some(w) = &mut self.parser_writer
        {
            let (_, line, col) = self.resolver
                .as_ref()
                .unwrap()
                .get_byte_line(byte_start)
                .expect("couldn't resolve location from byte offset");
            writeln!(w, "{}:{} error:{}", line + 1, col + 1, message)
                .expect("failed to write to parse file buffer");
            //detach after first report
            self.parser_writer = None;
        }
    }

    pub fn flush(&mut self) {
        if let Some(w) = &mut self.lexer_writer {
            w.flush().expect("failed to flush writer to lex file");
        }
        if let Some(w) = &mut self.parser_writer {
            w.flush().expect("failed to flush writer to parse file");
        }
    }
}
