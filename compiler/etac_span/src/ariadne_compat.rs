use std::ops::Range;

use crate::{FileId, SCache, Span};

/// Span that keeps track of its source cache.
///
/// This is really only for aridane error reporting since we're incabable of 
/// passing the global space context to it from outside the library.
pub struct ReportableSpan<'a> {
    cache: &'a SCache,
    pub span: Span,
    own: std::cell::OnceCell<(Range<u32>, FileId)>,
}

impl<'a> ReportableSpan<'a> {
    pub fn new(cache: &'a SCache, span: Span) -> Self {
        ReportableSpan {
            cache,
            span,
            own: std::cell::OnceCell::new(),
        }
    }
}

impl<'a> ariadne::Span for ReportableSpan<'a> {
    type SourceId = FileId;

    fn source(&self) -> &Self::SourceId {
        &self.own.get_or_init(|| self.cache.resolve_span(self.span)).1
    }

    fn start(&self) -> usize {
        self.own.get_or_init(|| self.cache.resolve_span(self.span)).0.start as usize
    }

    fn end(&self) -> usize {
        self.own.get_or_init(|| self.cache.resolve_span(self.span)).0.end as usize
    }
}
