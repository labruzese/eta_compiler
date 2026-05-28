use crate::cli::Flags;
use etac_span::FileId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

type WriterMap = HashMap<FileId, BufWriter<File>>;

pub struct Logger {
    lexer_writers: RefCell<WriterMap>,
    parser_writers: RefCell<WriterMap>,
    diag_root: PathBuf,
}

impl Logger {
    pub fn new(flags: &Flags) -> Self {
        if (flags.lex || flags.parse) && flags.diag_path != PathBuf::from("-") {
            std::fs::create_dir_all(&flags.diag_path)
                .expect("unable to create diagnostic output directory");
        }
        Self {
            lexer_writers: RefCell::new(HashMap::new()),
            parser_writers: RefCell::new(HashMap::new()),
            diag_root: flags.diag_path.clone(),
        }
    }

    pub fn log_token(
        &self,
        file: &FileId,
        at: (usize, usize),
        token: &impl std::fmt::Display,
    ) -> Result<(), std::io::Error> {
        let mut guard = self.lexer_writers.borrow_mut();
        let w = guard
            .entry(file.clone())
            .or_insert_with(|| open_log(&self.diag_root, file.as_str(), ".lexed"));
        writeln!(w, "{}:{} {}", at.0, at.1, token)
    }

    pub fn log_lexical_error(
        &self,
        file: &FileId,
        at: (usize, usize),
        message: &str,
    ) -> Result<(), std::io::Error> {
        let mut guard = self.lexer_writers.borrow_mut();
        let w = guard
            .entry(file.clone())
            .or_insert_with(|| open_log(&self.diag_root, file.as_str(), ".lexed"));
        writeln!(w, "{}:{} error:{}", at.0, at.1, message)
    }

    pub fn log_parse(
        &self,
        file: &FileId,
        program: impl std::fmt::Display,
    ) -> Result<(), std::io::Error> {
        let mut guard = self.parser_writers.borrow_mut();
        let w = guard
            .entry(file.clone())
            .or_insert_with(|| open_log(&self.diag_root, file.as_str(), ".parsed"));
        writeln!(w, "{}", program)
    }

    pub fn log_syntactic_error(
        &self,
        file: &FileId,
        at: (usize, usize),
        message: &str,
    ) -> Result<(), std::io::Error> {
        let mut guard = self.parser_writers.borrow_mut();
        let w = guard
            .entry(file.clone())
            .or_insert_with(|| open_log(&self.diag_root, file.as_str(), ".parsed"));
        writeln!(w, "{}:{} error:{}", at.0, at.1, message)
    }
}

fn open_log(root: &Path, file_name: &str, ext: &str) -> BufWriter<File> {
    let path = if root.eq(&PathBuf::from("-")) {
        PathBuf::from("/dev/stdout")
    } else {
        root.join(file_name).with_extension(ext)
    };

    BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .create_new(false)
            .open(path)
            .expect("unable to open diagnostic file"),
    )
}
