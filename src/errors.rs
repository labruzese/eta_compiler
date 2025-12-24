use codespan_reporting::{
    diagnostic::Diagnostic,
    files::SimpleFile,
    term::termcolor::{ColorChoice, StandardStream},
};

// include diagnostic creater for lexer
mod lexer;

pub struct ErrorHandler<'files> {
    context: SimpleFile<&'files str, &'files str>,
}

impl<'a> ErrorHandler<'a> {
    pub fn new(file: &'a str, source: &'a str) -> Self {
        Self {
            context: SimpleFile::new(file, source),
        }
    }

    pub fn show(&self, diagnostic: &Diagnostic<()>) {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();

        codespan_reporting::term::emit_to_write_style(
            &mut writer.lock(),
            &config,
            &self.context,
            &diagnostic,
        )
        .unwrap();
    }
}
