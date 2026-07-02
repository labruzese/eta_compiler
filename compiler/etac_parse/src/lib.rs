use etac_ast::{Expr, ExprKind, LValue, LValueKind, NodeIdGen};
use etac_errors::{etac_error, Diag, DiagCtxt};
use etac_lexer::Token;
use etac_span::Span;
use lalrpop_util::{lalrpop_mod, ErrorRecovery, ParseError};

lalrpop_mod!(grammar);

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
    Recovered(Out),
    /// Parsing hit a fatal error and produced no tree.
    Failed,
}

impl<Out> Parsed<Out> {
    /// The parsed tree, if one was produced ([`Ok`](Parsed::Ok) or
    /// [`Recovered`](Parsed::Recovered)).
    pub fn output(&self) -> Option<&Out> {
        match self {
            Parsed::Ok(out) | Parsed::Recovered(out) => Some(out),
            Parsed::Failed => None,
        }
    }
}

/// Mutable state threaded through every grammar action.
///
/// Bundles the [`NodeIdGen`] that hands out node ids with the buffer lalrpop fills
/// on error recovery, so the grammar carries a single parameter instead of two.
pub(crate) struct ParseState<'dcx, 'src> {
    pub diagc: &'dcx DiagCtxt<'src>,
    pub ids: NodeIdGen,
    pub lalrpop_errs: Vec<ErrorRecovery<u32, Token<'src>, Diag<'dcx, 'src>>>,
    pub etac_errs: Vec<Diag<'dcx, 'src>>,
}

impl<'dcx, 'src> ParseState<'dcx, 'src> {
    #[must_use]
    pub fn new(diagnostic_context: &'dcx DiagCtxt<'src>) -> Self {
        ParseState {
            diagc: diagnostic_context,
            ids: NodeIdGen::default(),
            lalrpop_errs: Vec::new(),
            etac_errs: Vec::new(),
        }
    }
}

pub use grammar::__ToTriple;

pub trait IParser<'dcx, 'src> {
    type Out: std::fmt::Display;

    fn parse<Lexer>(&mut self, lexer: &mut Lexer) -> Parsed<Self::Out>
    where
        Lexer: Iterator<Item = Result<(u32, Token<'src>, u32), Diag<'dcx, 'src>>>,
        'src: 'dcx; 

    fn errors_mut(&mut self) -> &mut [Diag<'dcx, 'src>];

    fn into_errors(self) -> Vec<Diag<'dcx, 'src>>;

    fn diagnostic_context(&self) -> &'dcx DiagCtxt<'src>;

}

/// Creates a new struct shadowing the name of the passed on (you must qualify a path ex:
/// [`grammar::ProgramParser`]) and implements [`IParser`] for it.
macro_rules! impl_iparser {
    ($($seg:ident)::+, $out:ty) => {
        impl_iparser!(@inner ($($seg)::+) ($($seg)::+) $out);
    };
    // strip the leading segment and keep going.
    (@inner ($full:path) ($head:ident :: $($rest:ident)::+) $out:ty) => {
        impl_iparser!(@inner ($full) ($($rest)::+) $out);
    };
    (@inner ($full:path) ($name:ident) $out:ty) => {
        pub struct $name<'dcx, 'src> {
            state: ParseState<'dcx, 'src>
        }
        impl<'dcx, 'src> $name<'dcx, 'src> {
            #[must_use]
            pub fn new(diagc: &'dcx DiagCtxt<'src>) -> Self { $name { state: ParseState::new(diagc) } }
        }
        impl<'dcx, 'src> IParser<'dcx, 'src> for $name<'dcx, 'src> {
            type Out = $out;

           fn parse<Lexer>(&mut self, lexer: &mut Lexer) -> Parsed<Self::Out>
            where
                Lexer: Iterator<Item = Result<(u32, Token<'src>, u32), Diag<'dcx, 'src>>>,
                'src: 'dcx
            {
                let parse = <$full>::parse(&<$full>::new(), &mut self.state, lexer);
                let mut recovered = false;
                for e in std::mem::take(&mut self.state.lalrpop_errs) {
                    let diag = to_diag(self.diagnostic_context(), e.error);
                    if diag.level == etac_errors::Level::Error {
                        recovered = true
                    }
                    self.state.etac_errs.push(diag)
                }
                match (parse, recovered) {
                    (Ok(out), false) => Parsed::Ok(out),
                    (Ok(out), true) => Parsed::Recovered(out),
                    (Err(fatal), _) => {
                        self.state.etac_errs.push(to_diag(self.diagnostic_context(), fatal));
                        Parsed::Failed
                    }
                }
            }

            fn errors_mut(&mut self) -> &mut [Diag<'dcx, 'src>] {
                &mut self.state.etac_errs
            }

            fn into_errors(self) -> Vec<Diag<'dcx, 'src>> {
                self.state.etac_errs
            }

            fn diagnostic_context(&self) -> &'dcx DiagCtxt<'src> {
                &self.state.diagc
            }
        }
    };
}

impl_iparser! {grammar::ProgramParser, etac_ast::Program}
impl_iparser! {grammar::InterfaceParser, etac_ast::Interface}


/// Reinterpret a parsed [`LValue`] as the equivalent [`Expr`], minting fresh ids for
/// the rebuilt carrier. The AST models the array operand of an indexed lvalue
/// (`a[i]`) as an `Expr`, so the grammar funnels the accumulated base through here
/// when folding postfix `[..]` groups.
pub(crate) fn lvalue_to_expr(lv: LValue, ids: &mut NodeIdGen) -> Expr {
    let kind = match lv.kind {
        LValueKind::Id(id) => ExprKind::Id(id),
        LValueKind::ProcCall(pc) => ExprKind::Call(pc),
        LValueKind::Index { array, index } => ExprKind::Index { array, index },
    };
    Expr::new(ids.fresh(), lv.span, kind)
}

fn to_diag<'dcx, 'src>(
    diagc: &'dcx DiagCtxt<'src>,
    err: ParseError<u32, Token<'src>, Diag<'dcx, 'src>>,
) -> Diag<'dcx, 'src> {
    use ParseError::*;
    match err {
        User { error } => error,

        UnrecognizedToken {
            token: (s, t, e),
            expected,
        } => {
            etac_error! {
                diagc, Span::new(s, e), "Unexpected token {}", t;
                primary: "{}", format_expected(&expected)
            }
        }
        UnrecognizedEof { location, expected } => {
            etac_error! {
                diagc, Span::new(location, location), "Unexpected end of file";
                primary: "{}", format_expected(&expected)
            }
        }
        ExtraToken { token: (s, t, e) } => {
            etac_error! {
                diagc, Span::new(s, e), "Extra token {} after program", t;
                primary: "unexpected"
            }
        }

        InvalidToken { location: _ } => {
            unreachable!("external lexer; lalrpop can not recieve an invalid token")
        }
    }
}

fn format_expected(expected: &[String]) -> String {
    match expected.len() {
        0 => String::from("expected nothing"),
        1 => format!("expected {}", expected[0]),
        _ => {
            let (last, rest) = expected.split_last().unwrap();
            format!("expected one of {}, or {}", rest.join(", "), last)
        }
    }
}
