//! Compiler diagnostics.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Denotes the severity of the Diagnostic
pub enum Level {
    Error,
    Warning,
    Note,
}

mod dcx;
mod emitter;
mod drop_bomb;
mod macros;

pub use dcx::*;
pub use emitter::*;
