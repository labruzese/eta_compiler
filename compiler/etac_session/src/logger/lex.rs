use std::{fs::File, io::BufWriter, io::Write};

use etac_errors::{Diag, Level};
use etac_lexer::{ILexer, Token};
use etac_span::{FileId, SourceCache};

use crate::logger::Logger;

/// Token-stream wrapper returned by [`Logger::tee`]. Forwards every item untouched and
/// logs as a side effect; see that method for the contract.
pub struct TeeLexer<I> {
    /// `None` when `--lex` is off: nothing is opened or written.
    writer: Option<BufWriter<File>>,
    source: &'static SourceCache,
    inner: I,
    stopped: bool,
}

impl<'dcx, I: ILexer<'dcx>> ILexer<'dcx> for TeeLexer<I> {}

impl<'dcx, I: ILexer<'dcx>> Iterator for TeeLexer<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.inner.next()?;
        if self.stopped {
            return Some(item);
        }
        let Some(writer) = self.writer.as_mut() else {
            return Some(item);
        };

        match &item {
            Ok((start, tok, _end)) => {
                if let Ok(at) = self.source.lc_index(*start) {
                    let _ = writeln!(writer, "{}:{} {}", at.0, at.1, tok);
                }
            }
            Err(diag) => {
                if diag.level == Level::Error {
                    if let Some(loc) = diag.loc.as_ref()
                        && let Ok(at) = self.source.lc_index(loc.lo)
                    {
                        let _ = writeln!(writer, "{}:{} error:{}", at.0, at.1, diag.message);
                    }
                    self.stopped = true;
                }
            }
        }

        Some(item)
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
    pub fn tee_lexer<'dcx, I>(&'dcx self, file: FileId, sources: &'static SourceCache, inner: I) -> TeeLexer<I>
    where
        I: Iterator<Item = Result<(u32, Token<'static>, u32), Diag<'dcx>>>,
    {
        TeeLexer {
            source: sources,
            writer: self
                .lex
                .then(|| super::open_log(&self.diag_root, file.as_str(), "lexed")),
            inner,
            stopped: false,
        }
    }
}
