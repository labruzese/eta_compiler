use std::cell::Cell;

use ariadne::Source;
use etac_span::{SourceCache, FileId};

pub struct Entry {
    id: FileId,
    name: String,
    content: String,
}

pub struct TestingCache {
    data: Cell<Vec<Entry>>
}

impl SourceCache for TestingCache {
    fn contains(&self, display_name: &str) -> Option<etac_span::FileId> {
        self.data.iter().find_map(|Entry {id, name, ..}| if name == display_name { Some(*id)} else { None })
    }

    fn store(&self, display_name: String, value: String) -> (etac_span::FileId, &Source<String>) {
        self.data.push
    }

    fn load_source(&self, id: etac_span::FileId) -> &ariadne::Source<String> {
        todo!()
    }

    fn load_name(&self, id: etac_span::FileId) -> &str {
        todo!()
    }

    fn resolve_span(&self, span: etac_span::Span) -> (std::ops::Range<u32>, etac_span::FileId) {
        todo!()
    }

    fn load(&self, id: etac_span::FileId) -> (u32, &ariadne::Source<String>) {
        todo!()
    }
}
