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
mod macros;
mod guarentee;

#[cfg(debug_assertions)]
mod drop_bomb;
