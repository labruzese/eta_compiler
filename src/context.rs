use crate::cli::Flags;
use crate::logger::Logger;
use crate::sources::Sources;

pub struct Context {
    pub flags: Flags,
    pub files: Sources,
    pub logger: Logger,
}

impl Context {
    pub fn new(flags: Flags) -> Self {
        let logger = Logger::new(&flags);
        let sources = Sources::new();
        Self { flags, files: sources, logger }
    }
}
