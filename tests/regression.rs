use generate_tests::generate_tests;

use std::path::{Path, PathBuf};
use std::process::Command;

fn find_source(sol: &Path) -> PathBuf {
    let dir = sol.parent().unwrap();
    let stem = sol.file_stem().unwrap().to_str().unwrap();
    ["eta", "eti", "rh"]
        .iter()
        .map(|ext| dir.join(format!("{stem}.{ext}")))
        .find(|p| p.exists())
        .unwrap_or_else(|| panic!("no source file for {}", sol.display()))
}

fn interpret(s: &str) -> SExpr {
    
    s.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn run_etac(flag: &str, source: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_etac"))
        .args([flag, "-D", "-", source.to_str().unwrap()])
        .output()
        .expect("failed to execute etac");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn check(flag: &str, sol_path: &Path) {
    let source = find_source(sol_path);
    let expected = normalize(&std::fs::read_to_string(sol_path).unwrap());
    let actual = normalize(&run_etac(flag, &source));
    similar_asserts::assert_eq!(actual, expected);
}

#[generate_tests(path = "tests/pa1", matches = r"\.lexedsol$")]
fn lex_test(input: &Path) {
    check("--lex", input);
}

#[generate_tests(path = "tests/pa2", matches = r"\.parsedsol$")]
fn parse_test(input: &Path) {
    check("--parse", input);
}

// fn typecheck_test(input: &Path) {
//     check("--typecheck", input);
// }
