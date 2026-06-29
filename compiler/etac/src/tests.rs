#[cfg(test)]
mod tests {
    use generate_tests::generate_tests;

    use std::path::{Path, PathBuf};
    use std::process::Command;

    mod sexpr;
    use sexpr::*;

    // ── Helpers ─────────────────────────────────────────────────────────

    fn find_source(sol: &Path) -> PathBuf {
        let dir = sol.parent().unwrap();
        let stem = sol.file_stem().unwrap().to_str().unwrap();
        ["eta", "eti", "rh"]
            .iter()
            .map(|ext| dir.join(format!("{stem}.{ext}")))
            .find(|p| p.exists())
            .unwrap_or_else(|| panic!("no source file for {}", sol.display()))
    }

    fn normalize(s: &str) -> String {
        s.lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    fn is_error_output(s: &str) -> bool {
        let trimmed = s.trim();
        // Error lines look like "3:5 error:..." — valid sexpr output starts with '('
        !trimmed.is_empty() && !trimmed.starts_with('(')
    }

    /// Extract the "line:col" prefix from an error string like "2:12 error:..."
    fn error_position(s: &str) -> &str {
        let s = s.trim();
        s.find(' ').map_or(s, |i| &s[..i])
    }

    fn run_etac(flag: &str, source: &Path) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_etac"))
            .args([flag, "-D", "-", source.to_str().unwrap()])
            .output()
            .expect("failed to execute etac");
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    // ── Test checkers ───────────────────────────────────────────────────

    fn check_lex(flag: &str, sol_path: &Path) {
        let source = find_source(sol_path);
        let expected = normalize(&std::fs::read_to_string(sol_path).unwrap());
        let actual = normalize(&run_etac(flag, &source));
        if actual != expected {
            let exp_lines: Vec<&str> = expected.lines().collect();
            let act_lines: Vec<&str> = actual.lines().collect();
            for (i, (e, a)) in exp_lines.iter().zip(act_lines.iter()).enumerate() {
                if e != a {
                    if let Some(ei) = e.find("error:") && 
                       let Some(ai) = a.find("error:") {
                        if e[..ei] == a[..ai] { continue }
                    }

                    let ctx_start = i.saturating_sub(2);
                    let ctx_end = (i + 3).min(exp_lines.len()).min(act_lines.len());
                    let mut msg = format!("first difference at line {}:\n", i + 1);
                    for l in ctx_start..ctx_end {
                        let marker = if l == i { ">>>" } else { "   " };
                        msg.push_str(&format!(
                            "{marker} expected: {}\n{marker}   actual: {}\n",
                            exp_lines.get(l).unwrap_or(&"<eof>"),
                            act_lines.get(l).unwrap_or(&"<eof>"),
                        ));
                    }
                    panic!("{msg}");
                }
            }
            if exp_lines.len() != act_lines.len() {
                panic!(
                    "output length differs: expected {} lines, got {} lines",
                    exp_lines.len(),
                    act_lines.len()
                );
            }
        }
    }

    fn check_parse(flag: &str, sol_path: &Path) {
        let source = find_source(sol_path);
        let expected_raw = std::fs::read_to_string(sol_path).unwrap();
        let actual_raw = run_etac(flag, &source);

        let expected_trimmed = expected_raw.trim();
        let actual_trimmed = actual_raw.trim();

        // If either side is an error message, just check that both are errors
        // at the same line:col — the message text doesn't matter.
        if is_error_output(expected_trimmed) || is_error_output(actual_trimmed) {
            if !is_error_output(expected_trimmed) {
                panic!(
                    "parse mismatch for {}:\n  expected sexpr but got error\n  actual: {actual_trimmed}",
                    sol_path.display(),
                );
            }
            if !is_error_output(actual_trimmed) {
                panic!(
                    "parse mismatch for {}:\n  expected error: {expected_trimmed}\n  but got sexpr:  {}",
                    sol_path.display(),
                    &actual_trimmed[..actual_trimmed.len().min(200)],
                );
            }
            let exp_pos = error_position(expected_trimmed);
            let act_pos = error_position(actual_trimmed);
            if exp_pos != act_pos {
                panic!(
                    "error location mismatch for {}:\n  expected: {expected_trimmed}\n    actual: {actual_trimmed}",
                    sol_path.display(),
                );
            }
            return;
        }

        let expected_sexpr = parse_sexpr(expected_trimmed)
            .unwrap_or_else(|e| panic!("bad expected sexpr in {}: {e}", sol_path.display()));
        let actual_sexpr = parse_sexpr(actual_trimmed)
            .unwrap_or_else(|e| panic!("bad actual sexpr for {}: {e}", sol_path.display()));

        if let Some(diff) = diff_sexpr(&expected_sexpr, &actual_sexpr) {
            panic!(
                "parse mismatch for {}:\n{diff}",
                sol_path.display(),
            );
        }
    }

    #[generate_tests(path = "../../etac_tests/pa1", matches = r"\.lexedsol$")]
    fn lex_test(input: &Path) {
        check_lex("--lex", input);
    }

    #[generate_tests(path = "../../etac_tests/pa2", matches = r"\.parsedsol$")]
    fn parse_test(input: &Path) {
        check_parse("--parse", input);
    }

    // #[generate_tests(path = "tests/pa3", matches = r"\.typedsol$")]
    // fn typecheck_test(input: &Path) {
    //     check_parse("--typecheck", input);
    // }
}
