#[derive(Debug)]
pub struct DropBomb(bool, &'static str);
impl DropBomb {
    pub fn new(msg: &'static str) -> Self {
        Self(true, msg)
    }
    pub fn defuse(&mut self) {
        self.0 = false
    }
}
impl Drop for DropBomb {
    fn drop(&mut self) {
        assert!(!self.0, "BOOM: {}", self.1);
    }
}
