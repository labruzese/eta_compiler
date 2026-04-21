use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtaSpan {
    pub file_id: FileId,
    pub range: std::ops::Range<usize>,
}

impl From<(FileId, std::ops::Range<usize>)> for EtaSpan {
    fn from((file_id, range): (FileId, std::ops::Range<usize>)) -> Self {
        EtaSpan { file_id, range }
    }
}

