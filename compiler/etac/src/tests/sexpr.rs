use std::fmt;
// ── S-expression tree ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sexpr {
    Atom(String),
    List(Vec<Sexpr>),
}

impl fmt::Display for Sexpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sexpr::Atom(s) => write!(f, "{s}"),
            Sexpr::List(xs) => {
                write!(f, "(")?;
                for (i, x) in xs.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    write!(f, "{x}")?;
                }
                write!(f, ")")
            }
        }
    }
}

#[allow(unused)]
impl Sexpr {
    /// Render this node with all children beyond depth 0 replaced by (…)
    pub fn elided(&self) -> String {
        match self {
            Sexpr::Atom(s) => s.clone(),
            Sexpr::List(xs) => {
                let inner: Vec<String> = xs.iter().map(|x| match x {
                    Sexpr::Atom(s) => s.clone(),
                    Sexpr::List(_) => "(…)".into(),
                }).collect();
                format!("({})", inner.join(" "))
            }
        }
    }
}

// ── Parser ──────────────────────────────────────────────────────────

pub fn parse_sexpr(input: &str) -> Result<Sexpr, String> {
    let tokens = tokenize(input)?;
    let (expr, rest) = parse_tokens(&tokens)?;
    if !rest.is_empty() {
        return Err(format!("trailing tokens: {:?}", &rest[..rest.len().min(5)]));
    }
    Ok(expr)
}

pub fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' => { chars.next(); }
            '(' | ')' => { tokens.push(c.to_string()); chars.next(); }
            '"' => {
                chars.next();
                let mut s = String::from('"');
                loop {
                    match chars.next() {
                        Some('\\') => {
                            s.push('\\');
                            if let Some(esc) = chars.next() { s.push(esc); }
                        }
                        Some('"') => { s.push('"'); break; }
                        Some(ch) => s.push(ch),
                        None => return Err("unterminated string".into()),
                    }
                }
                tokens.push(s);
            }
            '\'' => {
                // char literal like 'a'
                chars.next();
                let mut s = String::from('\'');
                loop {
                    match chars.next() {
                        Some('\\') => {
                            s.push('\\');
                            if let Some(esc) = chars.next() { s.push(esc); }
                        }
                        Some('\'') => { s.push('\''); break; }
                        Some(ch) => s.push(ch),
                        None => return Err("unterminated char literal".into()),
                    }
                }
                tokens.push(s);
            }
            _ => {
                let mut atom = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '(' || c == ')' || c.is_whitespace() { break; }
                    atom.push(c);
                    chars.next();
                }
                tokens.push(atom);
            }
        }
    }
    Ok(tokens)
}

pub fn parse_tokens<'a>(tokens: &'a [String]) -> Result<(Sexpr, &'a [String]), String> {
    match tokens.first().map(|s| s.as_str()) {
        None => Err("unexpected end of input".into()),
        Some("(") => {
            let mut rest = &tokens[1..];
            let mut children = Vec::new();
            loop {
                match rest.first().map(|s| s.as_str()) {
                    Some(")") => { rest = &rest[1..]; break; }
                    None => return Err("unclosed '('".into()),
                    _ => {
                        let (child, r) = parse_tokens(rest)?;
                        children.push(child);
                        rest = r;
                    }
                }
            }
            Ok((Sexpr::List(children), rest))
        }
        Some(")") => Err("unexpected ')'".into()),
        Some(_) => Ok((Sexpr::Atom(tokens[0].clone()), &tokens[1..])),
    }
}

pub struct SexprDiff {
    /// Lines that matched (printed as-is for context)
    pub matching_prefix: Vec<String>,
    pub matching_suffix: Vec<String>,   
    /// First expected line that diverged
    pub expected_line: String,
    /// First actual line that diverged
    pub actual_line: String,
    /// 1-indexed line number of the mismatch
    pub line_no: usize,
}

/// Pretty-print an Sexpr with indentation so we can diff line-by-line.
pub fn pretty_print(sexpr: &Sexpr, indent: usize) -> String {
    match sexpr {
        Sexpr::Atom(s) => s.clone(),
        Sexpr::List(xs) if xs.is_empty() => "()".into(),
        Sexpr::List(xs) => {
            // Try single-line first
            let one_line = format!("{sexpr}");
            if one_line.len() + indent <= 80 {
                return one_line;
            }
            // Otherwise break across lines
            let child_indent = indent + 1;
            let pad = " ".repeat(child_indent);
            let mut out = "(".to_string();
            for (i, x) in xs.iter().enumerate() {
                let child = pretty_print(x, child_indent);
                if i == 0 {
                    out.push_str(&child);
                } else {
                    out.push('\n');
                    out.push_str(&pad);
                    out.push_str(&child);
                }
            }
            out.push(')');
            out
        }
    }
}

pub fn diff_sexpr(expected: &Sexpr, actual: &Sexpr) -> Option<SexprDiff> {
    let exp_text = pretty_print(expected, 0);
    let act_text = pretty_print(actual, 0);

    let exp_lines: Vec<&str> = exp_text.lines().collect();
    let act_lines: Vec<&str> = act_text.lines().collect();

    for (i, (e, a)) in exp_lines.iter().zip(act_lines.iter()).enumerate() {
        if e != a {
            return Some(SexprDiff {
                matching_prefix: exp_lines[i.saturating_sub(BEFORE)..i]
                    .iter().map(|s| s.to_string()).collect(),
                matching_suffix: exp_lines[(i + 1).min(exp_lines.len())
                    ..(i + 1 + AFTER).min(exp_lines.len())]
                    .iter().map(|s| s.to_string()).collect(),
                expected_line: e.to_string(),
                actual_line: a.to_string(),
                line_no: i + 1,
            });
        }
    }

    if exp_lines.len() != act_lines.len() {
        let i = exp_lines.len().min(act_lines.len());
        return Some(SexprDiff {
            matching_prefix: exp_lines[i.saturating_sub(BEFORE)..i]
                .iter().map(|s| s.to_string()).collect(),
            matching_suffix: exp_lines[(i + 1).min(exp_lines.len())
                ..(i + 1 + AFTER).min(exp_lines.len())]
                .iter().map(|s| s.to_string()).collect(),
            expected_line: exp_lines.get(i).unwrap_or(&"<eof>").to_string(),
            actual_line: act_lines.get(i).unwrap_or(&"<eof>").to_string(),
            line_no: i + 1,
        });
    }

    None
}

/// Find the byte range where two strings differ.
fn mismatch_span(a: &str, b: &str) -> (usize, usize, usize, usize) {
    let common_prefix = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    let common_suffix = a.bytes().rev().zip(b.bytes().rev())
        .take_while(|(x, y)| x == y)
        .count()
        .min(a.len() - common_prefix)
        .min(b.len() - common_prefix);
    // (start, end) of the differing region in each string
    (common_prefix, a.len() - common_suffix, common_prefix, b.len() - common_suffix)
}

fn highlight_diff(label: &str, line: &str, start: usize, end: usize) -> String {
    format!(
        "{label}{}\x1b[31;1m{}\x1b[0m{}",
        &line[..start],
        &line[start..end],
        &line[end..],
    )
}

const BEFORE: usize = 5;
const AFTER:  usize = 3;

impl fmt::Display for SexprDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctx_start_line = self.line_no - self.matching_prefix.len();
        for (i, line) in self.matching_prefix.iter().enumerate() {
            writeln!(f, "  {:>4} │ {line}", ctx_start_line + i)?;
        }

        let (es, ee, as_, ae) =
            mismatch_span(&self.expected_line, &self.actual_line);

        writeln!(
            f, "  {:>4} │ {}",
            self.line_no,
            highlight_diff("\x1b[32mexp:\x1b[0m ", &self.expected_line, es, ee),
        )?;
        writeln!(
            f, "       │ {}",
            highlight_diff("\x1b[31mact:\x1b[0m ", &self.actual_line, as_, ae),
        )?;

        let suffix_start_line = self.line_no + 1;
        for (i, line) in self.matching_suffix.iter().enumerate() {
            writeln!(f, "  {:>4} │ {line}", suffix_start_line + i)?;
        }
        Ok(())
    }
}
