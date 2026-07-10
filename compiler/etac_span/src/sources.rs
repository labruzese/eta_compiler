use std::{fmt, ops::Range};

use ariadne::{Source};

use crate::{FileId, ReportableSpan, Span};

pub trait SourceCache: Send + Sync {
    fn contains(&self, display_name: &str) -> Option<FileId>;

    fn store(&mut self, display_name: String, value: String) -> (FileId, &Source<String>);

    fn load_source(&self, id: FileId) -> &ariadne::Source<String>; 

    fn load_name(&self, id: FileId) -> &str;

    fn resolve_span(&self, span: Span) -> (Range<u32>, FileId);

    fn reportable_span(&self, span: Span) -> ReportableSpan<'_, Self> {
        ReportableSpan::new(self, span)
    }
}

pub struct AriadneAdapter<'a, T>(pub &'a T);

impl<'a, T: SourceCache> ariadne::Cache<FileId> for AriadneAdapter<'a, T> {
    type Storage = String;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<Self::Storage>, impl fmt::Debug> {
        Ok::<_, std::convert::Infallible>(self.0.load_source(*id))
    }

    fn display<'b>(&self, id: &'b FileId) -> Option<impl fmt::Display + 'b> {
        Some(self.0.load_name(*id).to_owned())
    }
}

pub mod global_context;

// pub fn line_column(source: &ariadne::Source, at: usize) -> (u32, u32) {
//     let (_line, linen, coln) = source
//         .get_byte_line(at)
//         .map(|(a, b, c)| {
//             (
//                 a,
//                 u32::try_from(b).expect("requested line/col is out of bounds"),
//                 u32::try_from(c).expect("requested line/col is out of bounds"),
//             )
//         })
//         .expect("requested line/col is out of bounds");
//
//     (linen + 1, coln + 1)
// }
