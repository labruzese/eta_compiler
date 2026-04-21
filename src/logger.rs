use crate::{ast, cli};
use crate::sources::FileId;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::rc::Rc;

/// Handles printing the lexer log if enabled
pub struct Logger {
    lexer_writer: Option<BufWriter<std::fs::File>>,
    parser_writer: Option<BufWriter<std::fs::File>>,
    /// Requirements: Resolver is some iff any writer is some
    resolver: Option<ariadne::Source<Rc<str>>>,
}

impl Logger {
    pub fn new(options: &cli::Flags, file_id: FileId, source: Rc<str>) -> Self {
        let stem = std::path::Path::new(&file_id.to_string())
            .file_stem()
            .expect("source file has no file stem?")
            .to_os_string();

        let mut me = Self { 
            lexer_writer:  None,
            parser_writer:  None,
            resolver:  None,
        };
        if options.lex || options.parse {
            let _ = me.resolver.insert(ariadne::Source::from(source));
            std::fs::create_dir_all(&options.diag_path)
                .expect("unable to create diagnostic output directory");
        }
        if options.lex {
            let mut path = options.diag_path.clone();
            path.push(stem.clone());
            path = path.with_extension("lexed");
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .expect("unable to open lex file to write into");

            let _ = me.lexer_writer.insert(BufWriter::new(file));
        }    
        if options.parse {
            let mut path = options.diag_path.clone();
            path.push(stem);
            path = path.with_extension("parsed");
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .expect("unable to open parse file to write into");

            let _ = me.parser_writer.insert(BufWriter::new(file));
        }    
        me
    }

    pub fn is_logging_lexer(&self) -> bool { self.lexer_writer.is_some() }
    pub fn is_logging_parser(&self) -> bool { self.parser_writer.is_some() }

    pub fn log_token(&mut self, byte_start: usize, _byte_end: usize, token: &impl std::fmt::Display) {
        if let Some(w) = &mut self.lexer_writer
        {
            //these are 0 indexed
            let (_, line, col) = self.resolver
                .as_ref()
                .unwrap()
                .get_byte_line(byte_start)
                .expect("couldn't resolve location from byte offset");
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
