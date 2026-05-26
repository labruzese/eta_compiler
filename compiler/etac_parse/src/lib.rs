use etac_errors::{error, Diagnostic};
use etac_lexer::{Token};
use etac_span::{FileId}; 
use lalrpop_util::{lalrpop_mod, ParseError};

lalrpop_mod!(grammar);

pub trait IParser<ParseOut> {
    fn new() -> Self;

    fn parse<__TOKEN, __TOKENS>(
        &self,
        f: &etac_span::FileId,
        errors: &mut Vec<lalrpop_util::ErrorRecovery<usize, Token, Diagnostic>>,
        __tokens0: __TOKENS,
    ) -> Result<ParseOut, lalrpop_util::ParseError<usize, Token, Diagnostic>>
    where
        __TOKEN: grammar::__ToTriple,
        __TOKENS: IntoIterator<Item = __TOKEN>;
}

macro_rules! impl_iparser {
    ($parser:ty, $out:ty) => {
        impl IParser<$out> for $parser {
            fn new() -> Self { <$parser>::new() }

            fn parse<__TOKEN, __TOKENS>(
                &self,
                f: &etac_span::FileId,
                errors: &mut Vec<lalrpop_util::ErrorRecovery<usize, Token, Diagnostic>>,
                __tokens0: __TOKENS,
            ) -> Result<$out, lalrpop_util::ParseError<usize, Token, Diagnostic>>
            where
                __TOKEN: grammar::__ToTriple,
                __TOKENS: IntoIterator<Item = __TOKEN> {
                <$parser>::parse(self, f, errors, __tokens0)
            }
        }
    };
}

pub use grammar::ProgramParser;
pub use grammar::InterfaceParser;
impl_iparser!{ProgramParser, etac_ast::Program}
impl_iparser!{InterfaceParser, etac_ast::Interface}

pub fn parse<
    Out,
    Lexer,
    Parser,
    ParseCallback,
>(
    file_id: &FileId, // to generate diagnostics
    lexer: &mut Lexer,
    parse_cb: &mut ParseCallback,
) -> Result<Out, Vec<Diagnostic>>
where
    Lexer: Iterator<Item = Result<(usize, Token, usize), Diagnostic>>,
    Parser: IParser<Out>,
    ParseCallback: FnMut(&Result<Out, Diagnostic>),
{
    let mut recovered = Vec::new();
    let result = Parser::new()
                    .parse(file_id, &mut recovered, lexer)
                    .map_err(|e| to_diag(file_id, e));

    parse_cb(&result);

    match result {
        Ok(out) if recovered.is_empty() => return Ok(out),
        Ok(_)                           => Err(recovered.into_iter().map(|r| to_diag(file_id, r.error)).collect()),
        Err(e)                          => Err(recovered.into_iter().map(|r| to_diag(file_id, r.error)).chain(std::iter::once(e)).collect()),
    }
}

fn to_diag(file: &FileId, err: ParseError<usize, Token, Diagnostic>) -> Diagnostic {
    use ParseError::*;
    match err {
        User { error } => error,

        UnrecognizedToken {
            token: (s, t, e),
            expected,
        } => error!(file, s..e; "Unexpected token {t}")
            .with_primary_label(format_expected(&expected)),

        UnrecognizedEof { location, expected } => {
            error!(file, location..location; "Unexpected end of file")
                .with_primary_label(format_expected(&expected))
        }

        ExtraToken { token: (s, t, e) } => {
            error!(file, s..e; "Extra token {} after program", t).with_primary_label("unexpected")
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
