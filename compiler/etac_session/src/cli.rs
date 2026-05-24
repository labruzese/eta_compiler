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

    /// Source files to compile.
    #[arg(value_name = "SOURCE_FILES")]
    pub source_files: Vec<PathBuf>,
}

pub fn parse_flags() -> Flags {
    let flags = Flags::parse();

    if flags.source_files.is_empty() {
        let _ = Flags::command().print_help();
        std::process::exit(0);
    }

    flags
}

