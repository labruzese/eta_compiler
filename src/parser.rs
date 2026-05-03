use crate::ast;
use crate::context::Context;
use crate::error;
use crate::errors::Diagnostic;
use crate::lexer::Token;
use logos::Logos;
use crate::sources::{InterfaceId, SourceId};
use lalrpop_util::{lalrpop_mod, ParseError};

lalrpop_mod!(grammar);

macro_rules! parse {
    ($ctx:expr, $file_id:expr, $parser:ty) => {
        { 
            let source = $ctx
                .files
                .text($file_id)
                .map_err(|e| vec![e])?;

            let mut line_col = |offset: usize| $ctx.files.lc_index(&$file_id, offset).unwrap();

            let lexer = Token::lexer_with_extras(&source, $file_id.clone()).spanned();
            let tokens: Vec<Result<(usize, Token, usize), Diagnostic>> = lexer
                .map(|(tok, span)| match tok {
                    Ok(t) => {
                        $ctx.logger
                            .log_token(&$file_id, line_col(span.start), &t);
                        Ok((span.start, t, span.end))
                    }
                    Err(d) => {
                        $ctx.logger
                            .log_lexical_error(&$file_id, line_col(span.start), &d.message);
                        Err(d)
                    }
                })
                .collect();

            let mut recovered = Vec::new();
            let result = <$parser>::new().parse($file_id, &mut recovered, tokens);

            if result.is_err() || !recovered.is_empty() {
                let mut errors: Vec<Diagnostic> = recovered
                    .into_iter()
                    .map(|r| to_diag($file_id, r.error))
                    .collect();
                if let Err(e) = result {
                    errors.push(to_diag($file_id, e));
                }
                for err in &errors {
                    $ctx.logger
                        .log_syntactic_error(&$file_id, line_col(err.loc.range.start), &err.message);
                }
                return Err(errors);
            }

            let ast_node = result.unwrap();
            $ctx.logger.log_parse($file_id, &ast_node);
            Ok(ast_node) 
        }
    }
}

pub fn parse_source(ctx: &mut Context, file_id: &SourceId) -> Result<ast::Program, Vec<Diagnostic>> {
    parse!(ctx, file_id, grammar::ProgramParser)
}

pub fn parse_interface(ctx: &mut Context, file_id: &InterfaceId) -> Result<ast::Interface, Vec<Diagnostic>> {
    parse!(ctx, file_id, grammar::InterfaceParser)    
}

fn to_diag(file: &SourceId, err: ParseError<usize, Token, Diagnostic>) -> Diagnostic {
    use ParseError::*;
    match err {
        User { error } => error,

        UnrecognizedToken {
            token: (s, t, e),
            expected,
        } => error!(file, s..e, "Unexpected token {t}").with_primary_label(format_expected(&expected)),

        UnrecognizedEof { location, expected } => {
            error!(file, location..location, "Unexpected end of file")
                .with_primary_label(format_expected(&expected))
        }

        ExtraToken { token: (s, t, e) } => {
            error!(file, s..e, "Extra token {} after program", t).with_primary_label("unexpected")
        }

        InvalidToken { location: _ } => {
            unreachable!("external lexer; lalrpop can not recieve an invalid token")
        }
    }
}

fn format_expected(expected: &[String]) -> String {
    match expected.len() {
        0 => "expected nothing".into(),
        1 => format!("expected {}", expected[0]),
        _ => {
            let (last, rest) = expected.split_last().unwrap();
            format!("expected one of {}, or {}", rest.join(", "), last)
        }
    }
}
