use super::ErrorHandler;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use lalrpop_util::ParseError;

impl<'a> ErrorHandler<'a> {
    pub fn lexical_error<T: std::fmt::Display>(
        &self,
        error: ParseError<usize, T, &str>,
    ) -> Diagnostic<()> {
        match error {
            ParseError::UnrecognizedToken {
                token: (start, token, end),
                expected,
            } => Diagnostic::error()
                .with_message(format!("Unrecognized token: {}", token))
                .with_labels(vec![
                    Label::primary((), start..end).with_message("Unexpected token"),
                ])
                .with_notes(vec![format!("Expected one of: {}", expected.join(", "))]),

            ParseError::UnrecognizedEof { location, expected } => Diagnostic::error()
                .with_message("Unexpected end of file")
                .with_labels(vec![
                    Label::primary((), location..location).with_message("Expected more code here"),
                ])
                .with_notes(vec![format!("Expected one of: {}", expected.join(", "))]),

            ParseError::InvalidToken { location } => Diagnostic::error()
                .with_message("Invalid token")
                .with_labels(vec![
                    Label::primary((), location..location + 1)
                        .with_message("Failed to recognize this character"),
                ]),

            ParseError::ExtraToken {
                token: (start, token, end),
            } => Diagnostic::error()
                .with_message(format!("Extra token: {}", token))
                .with_labels(vec![
                    Label::primary((), start..end).with_message("Parser finished before this"),
                ]),

            //This doesn't exist yet, implement if you have custom errors in the lexer
            ParseError::User { error } => {
                Diagnostic::error().with_message(format!("User error: {}", error))
            }
        }
    }
}
