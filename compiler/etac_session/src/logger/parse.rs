use std::{fs::File, io::{BufWriter, Write}};

use etac_cache::{EtaCache, FileId};
use etac_errors::{Diag, Level};
use etac_lexer::ILexer;
use etac_parse::IParser;

use crate::logger::Logger;

pub struct TeeParser<'ec, I> {
    /// `None` when `--parse` is off: nothing is opened or written.
    writer: Option<BufWriter<File>>,
    cache: &'ec EtaCache,
    inner: I,
    stopped: bool,
}

impl<'ec, 'dcx, 'src, InnerParser> IParser<'dcx, 'src> for TeeParser<'ec, InnerParser>
where
    InnerParser: IParser<'dcx, 'src>,
    InnerParser::Out: std::fmt::Display,
{
    type Out = InnerParser::Out;

    fn parse(&mut self, lexer: &mut impl ILexer<'src, 'dcx>) -> etac_parse::Parsed<Self::Out> {
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
                let (loc, msg) = {
                    let diag = self
                        .inner
                        .errors_mut()
                        .iter()
                        .find(|d| d.level == Level::Error)
                        .expect("invariant of recovered");
                    (diag.loc, diag.message.clone())
                };
                if let Some(loc) = loc {
                    let (line, col) = self.cache.line_column(loc.lo);
                    let writer = self.writer.as_mut().expect("checked above");
                    let _ = writeln!(writer, "{line}:{col} error:{msg}");
                }
                self.stopped = true;
            }
        }

        result
    }

    fn errors_mut(&mut self) -> &mut [Diag<'dcx>] {
        self.inner.errors_mut()
    }

    fn into_errors(self) -> Vec<Diag<'dcx>> {
        self.inner.into_errors()
    }

    fn diagnostic_context(&self) -> &'dcx etac_errors::DiagCtxt<'dcx> {
        self.inner.diagnostic_context()
    }
}

impl Logger {
    /// Attach `--parse` logging to a parser.
    ///
    /// Returns a parser that behaves **identically** to `inner` while logging its output as
    /// a side effect: on a clean/recovered parse it writes the AST S-expression, and on
    /// failure it writes the first syntax error as `line:col error:<message>`. When parse
    /// logging is off the wrapper is a transparent pass-through, so the caller's type
    /// doesn't change with the flag.
    pub fn tee_parser<'ec, 'dcx, 'src, I>(&self, file: FileId<'ec>, cache: &'ec EtaCache, inner: I) -> TeeParser<'ec, I>
    where
        I: IParser<'dcx, 'src>,
    {
        TeeParser {
            cache,
            writer: self
                .parse
                .then(|| super::open_log(&self.diag_root, cache.source_name(file), "parsed")),
            inner,
            stopped: false,
        }
    }
}
