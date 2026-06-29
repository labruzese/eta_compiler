use etac_errors::{error, DiagCtxt, Diagnostic, ErrorGuaranteed};
use etac_lexer::{Token};
use etac_span::Span;
use lalrpop_util::{lalrpop_mod, ParseError};

lalrpop_mod!(grammar);

mod tests;

pub trait IParser<ParseOut> {
    fn new() -> Self;

    fn parse<__TOKEN, __TOKENS>(
        &self,
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
                errors: &mut Vec<lalrpop_util::ErrorRecovery<usize, Token, Diagnostic>>,
                __tokens0: __TOKENS,
            ) -> Result<$out, lalrpop_util::ParseError<usize, Token, Diagnostic>>
            where
                __TOKEN: grammar::__ToTriple,
                __TOKENS: IntoIterator<Item = __TOKEN> {
                <$parser>::parse(self, errors, __tokens0)
            }
        }
    };
}

pub use grammar::ProgramParser;
pub use grammar::InterfaceParser;
impl_iparser!{ProgramParser, etac_ast::Program}
impl_iparser!{InterfaceParser, etac_ast::Interface}

/// Outcome of a parse. Every diagnostic has already been emitted through the
/// [`DiagCtxt`] by the time this is returned — the caller never receives a `Vec` of
/// diagnostics to drain. The retained `first_error` exists only so the `.parsed` log
/// can record the first syntactic error for a file, and the [`ErrorGuaranteed`] is the
/// proof that the failure was reported.
#[derive(Debug)]
pub enum Parsed<Out> {
    /// Parsed cleanly, no errors.
    Ok(Out),
    /// lalrpop recovered from one or more errors but still produced a full tree.
    Recovered {
        out: Out,
        first_error: Diagnostic,
        guar: ErrorGuaranteed,
    },
    /// Parsing hit a fatal error and produced no tree.
    Failed {
        first_error: Diagnostic,
        guar: ErrorGuaranteed,
    },
}

impl<Out> Parsed<Out> {
    /// The parsed tree, if one was produced ([`Ok`](Parsed::Ok) or
    /// [`Recovered`](Parsed::Recovered)).
    pub fn output(&self) -> Option<&Out> {
        match self {
            Parsed::Ok(out) | Parsed::Recovered { out, .. } => Some(out),
            Parsed::Failed { .. } => None,
        }
    }

    /// The first error emitted while parsing, if any. Consumed by the `.parsed` logger.
    pub fn first_error(&self) -> Option<&Diagnostic> {
        match self {
            Parsed::Ok(_) => None,
            Parsed::Recovered { first_error, .. } | Parsed::Failed { first_error, .. } => {
                Some(first_error)
            }
        }
    }

    /// Proof that an error was reported, if this parse failed or recovered.
    pub fn error_guaranteed(&self) -> Option<ErrorGuaranteed> {
        match self {
            Parsed::Ok(_) => None,
            Parsed::Recovered { guar, .. } | Parsed::Failed { guar, .. } => Some(*guar),
        }
    }
}

/// Parse `lexer`'s tokens with `Parser`, routing every diagnostic through `dcx`.
///
/// lalrpop's recovered errors are emitted in source order, then any fatal error. The
/// first error is cloned and retained in the result purely for `.parsed` logging; the
/// caller inspects the [`Parsed`] variant (not a diagnostic list) to decide what to do.
pub fn parse<Out, Lexer, Parser>(dcx: &DiagCtxt, lexer: &mut Lexer) -> Parsed<Out>
where
    Lexer: Iterator<Item = Result<(usize, Token, usize), Diagnostic>>,
    Parser: IParser<Out>,
{
    let mut recovered = Vec::new();
    let result = Parser::new().parse(&mut recovered, lexer).map_err(to_diag);

    // Emit each recovered (non-fatal) error immediately, in source order, and remember
    // the first for logging.
    let mut first_error: Option<(Diagnostic, ErrorGuaranteed)> = None;
    for r in recovered {
        let diag = to_diag(r.error);
        let recorded = diag.clone();
        let guar = dcx.emit(diag).expect("syntax errors are always Level::Error");
        first_error.get_or_insert((recorded, guar));
    }

    match result {
        Ok(out) => match first_error {
            None => Parsed::Ok(out),
            Some((first_error, guar)) => Parsed::Recovered { out, first_error, guar },
        },
        Err(fatal) => {
            let recorded = fatal.clone();
            let guar = dcx.emit(fatal).expect("syntax errors are always Level::Error");
            let (first_error, guar) = first_error.unwrap_or((recorded, guar));
            Parsed::Failed { first_error, guar }
        }
    }
}

fn to_diag(err: ParseError<usize, Token, Diagnostic>) -> Diagnostic {
    use ParseError::*;
    match err {
        User { error } => error,

        UnrecognizedToken {
            token: (s, t, e),
            expected,
        } => error!(Span::new(s, e); "Unexpected token {t}")
            .with_primary_label(format_expected(&expected)),

        UnrecognizedEof { location, expected } => {
            error!(Span::new(location, location); "Unexpected end of file")
                .with_primary_label(format_expected(&expected))
        }

        ExtraToken { token: (s, t, e) } => {
            error!(Span::new(s, e); "Extra token {} after program", t).with_primary_label("unexpected")
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
