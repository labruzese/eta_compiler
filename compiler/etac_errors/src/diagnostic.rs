use super::{Level, Span, Color};

#[derive(Debug, Clone, PartialEq)]
/// Represents an emittable compiler diagnostic, usually these diagnostics have a location but
/// diagnostics such as io errors may not have a location to report.
/// In order to report any location information the diagnostic must have a primary label; see
/// [`with_primary_label`]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    pub loc: Option<Span>,
    pub labels: Vec<(Span, String, Color)>,
    pub code: Option<String>,
    pub note: Option<String>,
}

impl Diagnostic {
    /// Create a new diagnostic at a location with a message.
    pub fn new(level: Level, span: Span, message: impl Into<String>) -> Self {
        Self {
            level,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            loc: Some(span),
            note: None,
        }
    }

    /// Create a new diagnostic that doesn't have a location
    pub fn new_no_loc(level: Level, message: impl Into<String>) -> Self {
        Self {
            level,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            loc: None,
            note: None,
        }
    }

    /// Attach code to this diagnostic
    #[must_use = "diag is consumed before emission here"]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Attach a primary label to this diagnostic. This attaches the message as context pointing to 
    /// the span of the diagnostics location. This function *panics* if it is attached to a 
    /// diagnostic without a location.
    /// # Panics
    /// If this diagnostic does not have a location to attach a primary label to
    #[must_use = "diag is consumed before emission here"]
    pub fn with_primary_label(mut self, message: impl Into<String>) -> Self {
        self.labels.push((
            self.loc
                .unwrap_or_else(|| panic!("can not add primary label to a diagnostic without a location")),
            message.into(),
            Color::Red,
        ));
        self
    }

    /// Attach a label at a different location to this diagnostic.
    #[must_use = "diag is consumed before emission here"]
    pub fn with_secondary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push((span, message.into(), Color::Yellow));
        self
    }

    /// Attach a note about this diagnostic or about the problem to this diagnostic.
    #[must_use = "diag is consumed before emission here"]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Required by logos for the error type trait bound. Never actually used
/// because we always provide an explicit error callback.
impl Default for Diagnostic {
    fn default() -> Self {
        Self::new_no_loc(Level::Error, "unknown error")
    }
}

impl From<std::io::Error> for Diagnostic {
    fn from(value: std::io::Error) -> Self {
        error!("io error: {}", value.to_string())
    }
}
