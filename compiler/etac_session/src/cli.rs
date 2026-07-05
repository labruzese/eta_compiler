use clap::{CommandFactory, Parser};
use std::{path::PathBuf};

#[derive(Debug, Clone, Parser)]
#[command(name = "etac", about = "Rust Implementation of Eta Compiler")]
pub struct Flags {
    /// Generate output from lexical analysis.
    ///
    /// For each source file named filename.eta, a diagnostic output file
    /// named filename.lexed is generated.
    #[arg(short = 'l', long)]
    pub lex: bool,

    /// Generate output from syntactic analysis.
    ///
    /// For each source file named filename.eta, a diagnostic output file
    /// named filename.parsed is generated.
    ///
    /// If the source file is a syntactically invalid Eta program, the content of the .parsed file
    /// contains:
    /// <line>:<column> error:<description>
    /// where <line> and <column> indicate the beginning position of the error, and <description>
    /// details the error.
    ///
    /// If the source file is a syntactically valid Eta program, the content of the .parsed file contains
    /// an S-expression visualization of the AST representing the program.
    #[arg(short = 'p', long)]
    pub parse: bool,

    /// Specify where to place generated diagnostic files.
    ///
    /// The default is the current directory in which etac is run.
    #[arg(short = 'D', value_name = "PATH", default_value = ".")]
    pub diag_path: PathBuf,

    /// Specify where to find input source files.
    ///
    /// Relative source-file paths on the command line are resolved against
    /// this directory. The default is the current directory in which etac is
    /// run. The course-spec spelling `-sourcepath` is also accepted.
    #[arg(long = "sourcepath", value_name = "PATH", default_value = ".")]
    pub source_path: PathBuf,

    /// Specify where to find library interface files.
    ///
    /// A `use F` declaration searches for `F.eti` next to the using source
    /// file first, then in this directory. The default is the current
    /// directory in which etac is run. The course-spec spelling `-libpath`
    /// is also accepted.
    #[arg(long = "libpath", value_name = "PATH", default_value = ".")]
    pub lib_path: PathBuf,

    /// Source files to compile (relative paths resolve against --sourcepath).
    #[arg(value_name = "SOURCE_FILES")]
    pub source_files: Vec<PathBuf>,
}

#[must_use]
pub fn parse_flags() -> Flags {
    // The course spec spells the path flags with a single dash (-sourcepath,
    // -libpath), which clap cannot express: a single dash means bundled short
    // flags. Accept both spellings by normalizing argv before parsing.
    let args = std::env::args().map(|a| {
        for flag in ["sourcepath", "libpath"] {
            if a == format!("-{flag}") {
                return format!("--{flag}");
            }
            if let Some(rest) = a.strip_prefix(&format!("-{flag}=")) {
                return format!("--{flag}={rest}");
            }
        }
        a
    });
    let flags = Flags::parse_from(args);

    if flags.source_files.is_empty() {
        let _ = Flags::command().print_help();
        std::process::exit(0);
    }

    flags
}

