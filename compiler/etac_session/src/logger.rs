use crate::cli::Flags;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter};
use std::path::{Path, PathBuf};

pub mod lex;
pub mod parse;

/// Owns the external `--lex` / `--parse` log files and knows how to format each kind of
/// entry. Attach logging by tee'ing a phase
/// 
/// Logging is best-effort: whether a phase is being logged is decided here (from the
/// flags captured at construction), and I/O failures writing a log are swallowed rather
/// than ruining the token stream or aborting compilation.
pub struct Logger {
    diag_root: PathBuf,
    pub lex: bool,
    pub parse: bool,
}

impl Logger {
    /// # Panics
    /// If unable to create the diagnostic output directory
    #[must_use]
    pub fn new(flags: &Flags) -> Self {
        if (flags.lex || flags.parse) && flags.diag_path != *"-" {
            std::fs::create_dir_all(&flags.diag_path)
                .expect("unable to create diagnostic output directory");
        }
        Self {
            diag_root: flags.diag_path.clone(),
            lex: flags.lex,
            parse: flags.parse,
        }
    }
}

fn open_log(root: &Path, file_name: &str, ext: &str) -> BufWriter<File> {
    let path = if root.eq(&PathBuf::from("-")) {
        PathBuf::from("/dev/stdout")
    } else {
        let path = root.join(file_name).with_extension(ext);
        if let Some(parent) = path.parent() {
            // Log files mirror the source file's relative path under the
            // diagnostic root; make sure that subtree exists before opening.
            std::fs::create_dir_all(parent)
                .expect("unable to create diagnostic output directory");
        }
        path
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
