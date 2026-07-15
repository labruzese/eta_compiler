use std::ops::Range;

use crate::{EtaCache, FileId, Span};

/// A [`Span`] paired with the [`EtaCache`] that can resolve it.
///
/// Exists only for `ariadne` error rendering, which pulls the file id and
/// local offsets out of the span itself rather than taking a context.
pub struct ReportableSpan<'ec> {
    cache: &'ec EtaCache,
    pub span: Span,
    own: std::cell::OnceCell<(Range<u32>, FileId<'ec>)>,
}

impl<'ec> ReportableSpan<'ec> {
    pub fn new(cache: &'ec EtaCache, span: Span) -> Self {
        ReportableSpan {
            cache,
            span,
            own: std::cell::OnceCell::new(),
        }
    }

    fn resolved(&self) -> &(Range<u32>, FileId<'ec>) {
        self.own.get_or_init(|| self.cache.resolve_span(self.span))
    }
}

impl<'ec> ariadne::Span for ReportableSpan<'ec> {
    type SourceId = FileId<'ec>;

    fn source(&self) -> &Self::SourceId {
        &self.resolved().1
    }

    fn start(&self) -> usize {
        self.resolved().0.start as usize
    }

    fn end(&self) -> usize {
        self.resolved().0.end as usize
    }
}
