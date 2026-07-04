use etac_driver::{CompilationFailure, CompilationSuccess};

const ANSI_BOLD_GREEN: &str = "\x1b[1;32m";
const ANSI_BOLD_RED: &str = "\x1b[1;31m";
const ANSI_RESET: &str = "\x1b[0m";

fn main() -> std::process::ExitCode {
    env_logger::init();
    match etac_driver::run(&etac_session::cli::parse_flags()) {
        Ok(CompilationSuccess { warnings }) => {
            eprintln!("{ANSI_BOLD_GREEN}Compiled with {warnings} warnings");
            std::process::ExitCode::SUCCESS
        }
        Err(CompilationFailure { errors, warnings }) => {
            eprintln!("{ANSI_BOLD_RED}Compilation failed with {errors} errors and {warnings} warnings{ANSI_RESET}");
            std::process::ExitCode::FAILURE
        }
    }
}
