use etac_errors::{Diag, Level};
use etac_lexer::{ILexer};
use etac_parse::IParser;
use etac_span::{FileId, SourceCache};
use std::{fs::File, io::{BufWriter, Write}};

use crate::logger::Logger;

pub struct TeeParser<'src, I> {
    /// `None` when `--parse` is off: nothing is opened or written.
    writer: Option<BufWriter<File>>,
    source: &'src SourceCache,
    inner: I,
    stopped: bool,
}

impl<'dcx, 'src, InnerParser> IParser<'dcx, 'src> for TeeParser<'src, InnerParser>
where
    InnerParser: IParser<'dcx, 'src>,
    InnerParser::Out: std::fmt::Display,
{
    type Out = InnerParser::Out;

    fn parse(&mut self, lexer: &mut impl ILexer<'dcx, 'src>) -> etac_parse::Parsed<Self::Out>
    where 'src: 'dcx {
        let result = self.inner.parse(lexer);
        if self.stopped || self.writer.is_none() {
            return result;
        }

        match result {
            etac_parse::Parsed::Ok(ref out) => {
                let writer = self.writer.as_mut().expect("checked above");
                let _ = writeln!(writer, "{out}");
            }
            etac_parse::Parsed::Recovered(_) |
            etac_parse::Parsed::Failed => {
                let errors = self.errors_mut();
                let diag = errors.iter().find(|d| d.level == Level::Error).expect("invarient of recovered");
                if let Some(loc) = diag.loc {
                    let msg = diag.message.clone();
                    if let Ok((line, col)) = self.source.lc_index(loc.lo) {
                        let writer = self.writer.as_mut().expect("checked above");
                        let _ = writeln!(writer, "{line}:{col} error:{msg}");
                    }
                }
                self.stopped = true;
            }
        }

        result
    }

    fn errors_mut(&mut self) -> &mut [Diag<'dcx, 'src>] {
        self.inner.errors_mut()
    }

    fn into_errors(self) -> Vec<Diag<'dcx, 'src>> {
        self.inner.into_errors()
    }

    fn diagnostic_context(&self) -> &'dcx etac_errors::DiagCtxt<'src> {
        self.inner.diagnostic_context()
    }
}

impl Logger {
    /// Attach `--lex` logging to a token stream.
    ///
    /// Returns an iterator that yields `inner`'s items **unchanged** while logging each
    /// token (and the first lexical error) as a side effect. When lex logging is off the
    /// wrapper is a transparent pass-through, so the caller's type doesn't change with the
    /// flag. Per the Eta spec, logging stops at the first lexical error but the tokens
    /// keep flowing to the parser.
    pub fn tee_parser<'dcx, 'src, I>(&'dcx self, file: FileId, sources: &'src SourceCache, inner: I) -> TeeParser<'src, I>
    where
        I: IParser<'dcx, 'src>,
        'src: 'dcx
    {
        TeeParser {
            source: sources,
            writer: self
                .parse
                .then(|| super::open_log(&self.diag_root, file.as_str(), "parsed")),
            inner,
            stopped: false,
        }
    }
}
