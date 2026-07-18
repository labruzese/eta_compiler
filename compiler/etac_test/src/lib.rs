use std::fmt::Write as _;

use etac_cache::{EtaCache, FileId, Span};
use etac_errors::Diag;
use etac_lexer::Token;

pub fn write_diag(buffer: &mut String, diag: Diag<'_>, cache: &EtaCache) {
    let loc = diag.loc;
    let level = &diag.level;
    let message = &diag.message;
    let note = diag.note.as_deref().unwrap_or("");
    let mut labels = String::new();
    diag.labels.iter().for_each(|(span, message, ..)| {
        let file = cache.source_name(cache.resolve_span(*span).1);
        let (line_start, column_start) = cache.line_column(span.lo);
        let (line_end, column_end) = cache.line_column(span.hi);
        let _ = writeln!(labels, "\n\t{file}:{line_start}:{column_start}..{line_end}:{column_end} {message:?}");
    });
    let diag_str = format!("{level:?} {{\n\tmessage: {message}\n\tnote: {note}{labels}}}");
    match loc {
        Some(s) => {
            let file = cache.source_name(cache.resolve_span(s).1);
            let (line_start, column_start) = cache.line_column(s.lo);
            let (line_end, column_end) = cache.line_column(s.hi);
            let _ = writeln!(buffer, "{file}:{line_start}:{column_start}..{line_end}:{column_end} {diag_str}");
        }
        None => {
            let _ = writeln!(buffer, "{diag_str}");
        }
    }
    diag.cancel();
}

pub fn write_token(buffer: &mut String, token: (u32, Token, u32), cache: &EtaCache) {
    let (start, tok, end) = token;
    let file = cache.source_name(cache.resolve_span(Span::new(start, end)).1);
    let (line_start, column_start) = cache.line_column(start);
    let (line_end, column_end) = cache.line_column(end);
    let _ = writeln!(buffer, "{file}:{line_start}:{column_start}..{line_end}:{column_end} {tok:?}");
}

pub fn write_parse_output<T: etac_ast::sexpr::Sexpr>(buffer: &mut String, file: FileId, output: etac_parse::Parsed<T>, cache: &EtaCache) {
    let file = cache.source_name(file);
    match output.output() {
        Some(out) => {
            let spanctx = etac_ast::printer::spans::SpanCtx::new(|f| {
                let span = cache.span(f);
                let (line_start, column_start) = cache.line_column(span.lo);
                let (line_end, column_end) = cache.line_column(span.hi);
                Some(format!("{line_start}:{column_start}..{line_end}:{column_end}"))
            });
            let mut ast_str = String::new();
            let _ = out.to_doc(&etac_ast::sexpr::Plain).render_fmt(etac_ast::sexpr::WIDTH, &mut ast_str);
            let _ = writeln!(buffer, "{file}::ast {{\n{ast_str}\n}}");
        }
        None => {
            let _ = writeln!(buffer, "{file}::ast {{Failed}}");
        }
    }
}
