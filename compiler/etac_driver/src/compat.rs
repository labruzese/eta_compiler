use etac_errors::Diag;
use etac_lexer::ILexer;
use etac_parse::IParser;
use etac_session::logger::{lex::TeeLexer, parse::TeeParser};

/// A wrapper that holds one of the possible lexers that etac can have
pub enum ULexer<'src, I> {
    Raw(I),
    Tee(TeeLexer<'src, I>),
}

impl<'dcx, 'src, I: ILexer<'dcx, 'src>> ILexer<'dcx, 'src> for ULexer<'src, I>
where 'src: 'dcx {}

impl<'dcx, 'src, I: ILexer<'dcx, 'src>> Iterator for ULexer<'src, I>
where 'src: 'dcx {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ULexer::Raw(lexer) => lexer.next(),
            ULexer::Tee(lexer) => lexer.next(),
        }
    }
}


// A wrapper that holds one of the possible parser that etac can have
pub enum UParser<'src, I> {
    Raw(I),
    Tee(TeeParser<'src, I>),
}

impl<'dcx, 'src, I: IParser<'dcx, 'src>> IParser<'dcx, 'src> for UParser<'src, I>
where 
    I::Out: std::fmt::Display,
    'src: 'dcx 
{
    type Out = I::Out;

    fn parse(&mut self, lexer: &mut impl ILexer<'dcx, 'src>) -> etac_parse::Parsed<Self::Out>
    where 'src: 'dcx {
        match self {
            UParser::Raw(parser) => parser.parse(lexer),
            UParser::Tee(parser) => parser.parse(lexer),
        }
    }

    fn errors_mut(&mut self) -> &mut [Diag<'dcx, 'src>] {
        match self {
            UParser::Raw(parser) => parser.errors_mut(),
            UParser::Tee(parser) => parser.errors_mut(),
        }
    }

    fn into_errors(self) -> Vec<Diag<'dcx, 'src>> {
        match self {
            UParser::Raw(parser) => parser.into_errors(),
            UParser::Tee(parser) => parser.into_errors(),
        }
    }

    fn diagnostic_context(&self) -> &'dcx etac_errors::DiagCtxt<'src> {
        match self {
            UParser::Raw(parser) => parser.diagnostic_context(),
            UParser::Tee(parser) => parser.diagnostic_context(),
        }
    }
}
