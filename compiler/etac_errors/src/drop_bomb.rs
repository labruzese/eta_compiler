#[derive(Debug)]
pub struct DropBomb(bool);
impl DropBomb {
    pub fn new() -> Self {
        Self(true)
    }
    pub fn defuse(&mut self) {
        self.0 = false
    }
}
impl Drop for DropBomb {
    fn drop(&mut self) {
        assert!(!self.0, "Diag dropped without `.emit()`/`.cancel()`: {self:?}");
    }
}
