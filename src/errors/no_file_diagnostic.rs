/// This represents the diagnostic information of code in some file. This is
/// useful for when a file-agnostic part of our code wants to report a
/// diagnostic. Before we can emit a diagnostic we need to specify_file so the
/// actual reporting will be done after some code that knows what file we are
/// in can turn this into a diagnostic
use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct NoFileDiagnostic {
    pub level: Level,
    pub code: Option<String>,
    pub message: String,
    pub labels: Vec<(Range<usize>, String, Color)>,
    pub loc: Range<usize>,
    pub note: Option<String>,
}

impl NoFileDiagnostic {
    pub fn error(span: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            loc: span,
            note: None,
        }
    }

    pub fn warning(span: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Warning,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            loc: span,
            note: None,
        }
    }

    pub fn note(span: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Note,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            loc: span,
            note: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_primary_label(mut self, message: impl Into<String>) -> Self {
        self.labels.push((self.loc.clone(), message.into(), Color::Red));
        self
    }

    pub fn with_secondary_label(mut self, span: &Range<usize>, message: impl Into<String>) -> Self {
        self.labels
            .push((span.clone(), message.into(), Color::Yellow));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn specify_file(self, source: Source) -> Diagnostic {
        Diagnostic {
            level: self.level,
            code: self.code,
            message: self.message,
            labels: self
                .labels
                .into_iter()
                .map(|(r, l, c)| {
                    (
                        EtaSpan {
                            source: source.clone(),
                            range: r,
                        },
                        l,
                        c,
                    )
                })
                .collect(),
            loc: EtaSpan {
                source, 
                range: self.loc
            },
            note: self.note,
        }
    }
}

impl Default for NoFileDiagnostic {
    fn default() -> Self {
        Self::error((0usize..0), "Default error message")
    }
}
