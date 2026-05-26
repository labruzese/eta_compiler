use crate::cli::Flags;
use etac_span::FileId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

type WriterMap = HashMap<FileId, Option<BufWriter<File>>>;

pub struct Logger {
    lexer_writers: RefCell<Option<WriterMap>>,
    parser_writers: RefCell<Option<WriterMap>>,
    diag_root: PathBuf,
}

impl Logger {
    pub fn new(flags: &Flags) -> Self {
        if (flags.lex || flags.parse) 
        && flags.diag_path != PathBuf::from("-") {
            std::fs::create_dir_all(&flags.diag_path)
                .expect("unable to create diagnostic output directory");
        }
        Self {
            lexer_writers: RefCell::new(flags.lex.then(HashMap::new)),
            parser_writers: RefCell::new(flags.parse.then(HashMap::new)),
            diag_root: flags.diag_path.clone(),
        }
    }

    pub fn lex_enabled(&self) -> bool {
        self.lexer_writers.borrow_mut().as_mut().is_some()
    }

    pub fn parse_enabled(&self) -> bool {
        self.parser_writers.borrow_mut().as_mut().is_some()
    }

    /// Run `f` against the writer for `file` in `bucket`, creating it on
    /// first use. No-op if the bucket is disabled (`None`) or the file's
    /// writer was removed (after an error).
    fn with_writer(
        bucket: &RefCell<Option<WriterMap>>,
        diag_root: &Path,
        file: &FileId,
        ext: &'static str,
        f: impl FnOnce(&mut BufWriter<File>),
    ) {
        let mut guard = bucket.borrow_mut();
        let Some(writers) = guard.as_mut() else { return; };
        match writers.get_mut(file) {
            Some(Some(w)) => f(w),
            Some(None) => (),
            None => { 
                let mut w = open_log(diag_root, file.as_str(), ext);
                f(&mut w);
                writers.insert(file.clone(), Some(w)); 
            },
        }
    }

    pub fn log_token(
        &self,
        file: &FileId,
        at: (usize, usize),
        token: &impl std::fmt::Display,
    ) {
        Self::with_writer(&self.lexer_writers, &self.diag_root, file, "lexed", |w| {
            writeln!(w, "{}:{} {}", at.0, at.1, token).unwrap();
        });
    }

    pub fn log_lexical_error(&self, file: &FileId, at: (usize, usize), message: &str) {
        Self::with_writer(&self.lexer_writers, &self.diag_root, file, "lexed", |w| {
            writeln!(w, "{}:{} error:{}", at.0, at.1, message).unwrap();
        });
        let mut guard = self.lexer_writers.borrow_mut();
        let Some(writers) = guard.as_mut() else { return; };
        writers.insert(file.clone(), None); // remove writer (only report first error)
    }

    pub fn log_parse(&self, file: &FileId, program: impl std::fmt::Display) {
        Self::with_writer(&self.parser_writers, &self.diag_root, file, "parsed", |w| {
            writeln!(w, "{}", program).unwrap();
        });
    }

    pub fn log_syntactic_error(&self, file: &FileId, at: (usize, usize), message: &str) {
        Self::with_writer(&self.parser_writers, &self.diag_root, file, "parsed", |w| {
            writeln!(w, "{}:{} error:{}", at.0, at.1, message).unwrap();
        });
        let mut guard = self.parser_writers.borrow_mut();
        let Some(writers) = guard.as_mut() else { return; };
        writers.insert(file.clone(), None); // remove writer (only report first error)
    }
}

fn open_log(root: &Path, file_name: &str, ext: &str) -> BufWriter<File> {
    let path = if root.eq(&PathBuf::from("-")){
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
