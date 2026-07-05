use etac_errors::DiagCtxt;

#[derive(Debug)]
pub struct CompilationFailure {
    pub errors: usize,
    pub warnings: usize,
}
impl From<&DiagCtxt> for CompilationFailure {
    fn from(value: &DiagCtxt) -> Self {
        CompilationFailure {
            errors: value.err_count(),
            warnings: value.warn_count(),
        }
    }
}
pub struct CompilationSuccess {
    pub warnings: usize
}
impl From<&DiagCtxt> for CompilationSuccess {
    fn from(value: &DiagCtxt) -> Self {
        CompilationSuccess {
            warnings: value.warn_count(),
        }
    }
}
