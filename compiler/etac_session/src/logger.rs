use crate::cli::Flags;
use etac_errors::{Diagnostic, Level};
use etac_lexer::Token;
use etac_span::{FileId, SourceCache};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

type WriterMap = HashMap<FileId, BufWriter<File>>;

/// Owns the external `--lex` / `--parse` log files and knows how to format each kind of
/// entry. Phases attach logging in a single call ([`tee`](Logger::tee) for the token
/// stream, [`log_tree`](Logger::log_tree) / [`log_syntax_error`](Logger::log_syntax_error)
/// for parse output) and never format log lines themselves.
///
/// Logging is best-effort: whether a phase is being logged is decided here (from the
/// flags captured at construction), and I/O failures writing a log are swallowed rather
/// than perturbing the token stream or aborting compilation. Adding a new logged phase
/// (e.g. typecheck) is one method here plus one call site.
pub struct Logger {
    lexer_writers: RefCell<WriterMap>,
    parser_writers: RefCell<WriterMap>,
    diag_root: PathBuf,
    lex: bool,
    parse: bool,
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
            lex: flags.lex,
            parse: flags.parse,
        }
    }

    /// Attach `--lex` logging to a token stream.
    ///
    /// Returns an iterator that yields `inner`'s items **unchanged** while logging each
    /// token (and the first lexical error) as a side effect. When lex logging is off the
    /// wrapper is a transparent pass-through, so the caller's type doesn't change with the
    /// flag. Per the Eta spec, logging stops at the first lexical error but the tokens
    /// keep flowing to the parser.
    pub fn tee<'a, I>(&'a self, file: &'a FileId, sources: &'a SourceCache, inner: I) -> Tee<'a, I>
    where
        I: Iterator<Item = Result<(usize, Token, usize), Diagnostic>>,
    {
        Tee { logger: self, file, sources, inner, stopped: false }
    }

    /// Write a parsed tree to the `.parsed` log (no-op unless `--parse`). Best-effort.
    pub fn log_tree(&self, file: &FileId, tree: &impl std::fmt::Display) {
        if self.parse {
            let _ = self.write_parse(file, tree);
        }
    }

    /// Write the first syntactic error to the `.parsed` log (no-op unless `--parse`).
    /// Best-effort; a missing location or a log I/O failure is silently skipped.
    pub fn log_syntax_error(&self, file: &FileId, sources: &SourceCache, diag: &Diagnostic) {
        if !self.parse {
            return;
        }
        if let Some(loc) = diag.loc.as_ref()
            && let Ok(at) = sources.lc_index(loc.lo)
        {
            let _ = self.write_syntactic_error(file, at, &diag.message);
        }
    }

    // --- low-level writers (formatting lives here; callers use the methods above) ---

    fn write_token(
        &self,
        file: &FileId,
        at: (usize, usize),
        token: &impl std::fmt::Display,
    ) -> std::io::Result<()> {
        let mut guard = self.lexer_writers.borrow_mut();
        let w = guard
            .entry(file.clone())
            .or_insert_with(|| open_log(&self.diag_root, file.as_str(), ".lexed"));
        writeln!(w, "{}:{} {}", at.0, at.1, token)
    }

    fn write_lexical_error(
        &self,
        file: &FileId,
        at: (usize, usize),
        message: &str,
    ) -> std::io::Result<()> {
        let mut guard = self.lexer_writers.borrow_mut();
        let w = guard
            .entry(file.clone())
            .or_insert_with(|| open_log(&self.diag_root, file.as_str(), ".lexed"));
        writeln!(w, "{}:{} error:{}", at.0, at.1, message)
    }

    fn write_parse(&self, file: &FileId, program: &impl std::fmt::Display) -> std::io::Result<()> {
        let mut guard = self.parser_writers.borrow_mut();
        let w = guard
            .entry(file.clone())
            .or_insert_with(|| open_log(&self.diag_root, file.as_str(), ".parsed"));
        writeln!(w, "{}", program)
    }

    fn write_syntactic_error(
        &self,
        file: &FileId,
        at: (usize, usize),
        message: &str,
    ) -> std::io::Result<()> {
        let mut guard = self.parser_writers.borrow_mut();
        let w = guard
            .entry(file.clone())
            .or_insert_with(|| open_log(&self.diag_root, file.as_str(), ".parsed"));
        writeln!(w, "{}:{} error:{}", at.0, at.1, message)
    }
}

/// Token-stream wrapper returned by [`Logger::tee`]. Forwards every item untouched and
/// logs as a side effect; see that method for the contract.
pub struct Tee<'a, I> {
    logger: &'a Logger,
    file: &'a FileId,
    sources: &'a SourceCache,
    inner: I,
    /// Set once a lexical error has been seen: logging stops, forwarding continues.
    stopped: bool,
}

impl<I> Iterator for Tee<'_, I>
where
    I: Iterator<Item = Result<(usize, Token, usize), Diagnostic>>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.inner.next()?;

        if self.logger.lex && !self.stopped {
            match &item {
                Ok((start, tok, _end)) => {
                    if let Ok(at) = self.sources.lc_index(*start) {
                        // best-effort: a log write failure must not alter the token stream
                        let _ = self.logger.write_token(self.file, at, tok);
                    }
                }
                Err(diag) => {
                    if diag.level == Level::Error
                        && let Some(loc) = diag.loc.as_ref()
                        && let Ok(at) = self.sources.lc_index(loc.lo)
                    {
                        let _ = self.logger.write_lexical_error(self.file, at, &diag.message);
                    }
                    // Eta spec: lexing logging halts at the first error.
                    self.stopped = true;
                }
            }
        }

        Some(item)
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
