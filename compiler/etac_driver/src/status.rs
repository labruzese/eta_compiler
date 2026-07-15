use etac_errors::DiagCtxt;

#[derive(Debug)]
pub struct CompilationFailure {
    pub errors: usize,
    pub warnings: usize,
}
impl From<&DiagCtxt<'_>> for CompilationFailure {
    fn from(value: &DiagCtxt<'_>) -> Self {
        CompilationFailure {
            errors: value.err_count(),
            warnings: value.warn_count(),
        }
    }
}
pub struct CompilationSuccess {
    pub warnings: usize
}
impl From<&DiagCtxt<'_>> for CompilationSuccess {
    fn from(value: &DiagCtxt<'_>) -> Self {
        CompilationSuccess {
            warnings: value.warn_count(),
        }
    }
}
