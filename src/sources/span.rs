use super::*;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtaSpan {
    pub file_id: SourceId,
    pub range: std::ops::Range<usize>,
}

impl EtaSpan {
    pub fn new(file_id: SourceId, l: usize, r: usize) -> Self {
        Self { file_id, range: (l..r) }
    }
}

impl From<(&SourceId, std::ops::Range<usize>)> for EtaSpan {
    fn from((file_id, range): (&SourceId, std::ops::Range<usize>)) -> Self {
        EtaSpan { file_id: file_id.clone(), range }
    }
}

