use clap::{CommandFactory, Parser};
use std::{path::PathBuf, sync::OnceLock};

#[derive(Debug, Clone, Parser)]
#[command(name = "etac", about = "Rust Implementation of Eta Compiler")]
pub struct Flags {
    /// Generate output from lexical analysis.
    ///
    /// For each source file named filename.eta, a diagnostic output file
    /// named filename.lexed is generated.
    #[arg(long)]
    pub lex: bool,

    /// Specify where to place generated diagnostic files.
    ///
    /// The default is the current directory in which etac is run.
    #[arg(short = 'D', value_name = "PATH", default_value = ".")]
    pub diag_path: PathBuf,

    /// Source files to compile.
    #[arg(value_name = "SOURCE_FILES")]
    pub source_files: Vec<PathBuf>,
}

/// Global flags for this program
static FLAGS: OnceLock<Flags> = OnceLock::new();

/// Parses arguments from CLI and initializes the global FLAGS.
pub fn init() {
    let flags = Flags::parse();

    // Requirement: Invoking etac without any source files should also print a synopsis.
    if flags.source_files.is_empty() {
        let _ = Flags::command().print_help();
        std::process::exit(0);
    }

    FLAGS.set(flags).expect("Flags already initialized");
}

pub fn flags() -> &'static Flags {
    FLAGS.get().expect("Flags not initialized")
}
