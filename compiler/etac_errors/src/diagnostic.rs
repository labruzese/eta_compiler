use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub level: Level,
    pub code: Option<String>,
    pub message: String,
    pub labels: Vec<(EtaSpan, String, Color)>,
    pub file: Option<FileId>,
    pub loc: Option<EtaSpan>,
    pub note: Option<String>,
}

impl Diagnostic {
    pub fn new(level: Level, span: EtaSpan, message: impl Into<String>) -> Self {
        Self {
            level,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            file: Some(span.file_id.clone()),
            loc: Some(span),
            note: None,
        }
    }

    pub fn new_no_loc(level: Level, file: FileId, message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            file: Some(file),
            loc: None,
            note: None,
        }
    }

    pub fn new_generic(level: Level, message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            file: None,
            loc: None,
            note: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_primary_label(mut self, message: impl Into<String>) -> Self {
        self.labels.push((
            self.loc.clone().unwrap_or_else(||panic!("can not add primary label to a diagnostic without a location")),
            message.into(),
            Color::Red
        ));
        self
    }

    pub fn with_secondary_label(mut self, span: EtaSpan, message: impl Into<String>) -> Self {
        self.labels.push((span, message.into(), Color::Yellow));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Required by logos for the error type trait bound. Never actually used
/// because we always provide an explicit error callback.
impl Default for Diagnostic {
    fn default() -> Self {
        Self::new_generic(
            Level::Error,
            "unknown error",
        )
    }
}

impl From<std::io::Error> for Diagnostic {
    fn from(value: std::io::Error) -> Self {
        error!("io error: {}", value.to_string())
    }
}
