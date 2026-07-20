// Proof that an diagnostic was recorded in the diagnostic context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorGuaranteed(());

impl ErrorGuaranteed {
    /// Private so it can only originate on a real error path in this module.
    #[inline]
    pub(crate) fn new() -> Self {
        ErrorGuaranteed(())
    }
    /// Assert that an error was already reported elsewhere, without emitting one here.
    ///
    /// # Safety
    /// The compiler can't prove that an error was actually reported when constructed this way.
    /// You're responsible for making sure that actually emit the error to the user.
    #[inline]
    #[must_use]
    pub unsafe fn claim_already_emitted() -> Self {
        ErrorGuaranteed(())
    }
}
