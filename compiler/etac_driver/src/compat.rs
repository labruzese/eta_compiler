use etac_errors::Diag;
use etac_lexer::ILexer;
use etac_parse::IParser;
use etac_session::logger::{lex::TeeLexer, parse::TeeParser};

/// A wrapper that holds one of the possible lexers that etac can have
pub enum ULexer<I> {
    Raw(I),
    Tee(TeeLexer<I>),
}

impl<'dcx, 'src, I: ILexer<'dcx, 'src>> ILexer<'dcx, 'src> for ULexer<I> {}

impl<'dcx, 'src, I: ILexer<'dcx, 'src>> Iterator for ULexer<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ULexer::Raw(lexer) => lexer.next(),
            ULexer::Tee(lexer) => lexer.next(),
        }
    }
}


/// A wrapper that holds one of the possible parsers that etac can have
pub enum UParser<I> {
    Raw(I),
    Tee(TeeParser<I>),
}

impl<'dcx, 'src, I> IParser<'dcx, 'src> for UParser<I>
where
    I: IParser<'dcx, 'src> ,
    I::Out: std::fmt::Display,
{
    type Out = I::Out;

    fn parse(&mut self, lexer: &mut impl ILexer<'dcx, 'src>) -> etac_parse::Parsed<Self::Out> {
        match self {
            UParser::Raw(parser) => parser.parse(lexer),
            UParser::Tee(parser) => parser.parse(lexer),
        }
    }

    fn errors_mut(&mut self) -> &mut [Diag<'dcx>] {
        match self {
            UParser::Raw(parser) => parser.errors_mut(),
            UParser::Tee(parser) => parser.errors_mut(),
        }
    }

    fn into_errors(self) -> Vec<Diag<'dcx>> {
        match self {
            UParser::Raw(parser) => parser.into_errors(),
            UParser::Tee(parser) => parser.into_errors(),
        }
    }

    fn diagnostic_context(&self) -> &'dcx etac_errors::DiagCtxt {
        match self {
            UParser::Raw(parser) => parser.diagnostic_context(),
            UParser::Tee(parser) => parser.diagnostic_context(),
        }
    }
}
