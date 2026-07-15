use std::{fs::File, io::BufWriter, io::Write};

use etac_cache::{EtaCache, FileId};
use etac_errors::{Diag, Level};
use etac_lexer::{ILexer, Token};

use crate::logger::Logger;

/// Token-stream wrapper returned by [`Logger::tee_lexer`]. Forwards every item
/// untouched and logs as a side effect; see that method for the contract.
pub struct TeeLexer<'ec, I> {
    /// `None` when `--lex` is off: nothing is opened or written.
    writer: Option<BufWriter<File>>,
    cache: &'ec EtaCache,
    inner: I,
    stopped: bool,
}

impl<'ec, 'src, 'dcx, I: ILexer<'src, 'dcx>> ILexer<'src, 'dcx> for TeeLexer<'ec, I> {}

impl<'ec, 'src, 'dcx, I: ILexer<'src, 'dcx>> Iterator for TeeLexer<'ec, I> {
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
                let at = self.cache.line_column(*start);
                let _ = writeln!(writer, "{}:{} {}", at.0, at.1, tok);
            }
            Err(diag) => {
                if diag.level == Level::Error {
                    if let Some(loc) = diag.loc.as_ref() {
                        let at = self.cache.line_column(loc.lo);
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
    pub fn tee_lexer<'ec, 'src, 'dcx, I>(&self, file: FileId<'ec>, cache: &'ec EtaCache, inner: I) -> TeeLexer<'ec, I>
    where
        I: Iterator<Item = Result<(u32, Token<'src>, u32), Diag<'dcx>>>,
    {
        TeeLexer {
            cache,
            writer: self
                .lex
                .then(|| super::open_log(&self.diag_root, cache.source_name(file), "lexed")),
            inner,
            stopped: false,
        }
    }
}
