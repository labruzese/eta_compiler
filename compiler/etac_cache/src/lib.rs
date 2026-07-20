pub mod sources;    
pub mod asts;       

#[derive(Default)]
pub struct EtaCache {
    pub asts: asts::AstArena,
    pub sources: sources::SourceMap,
}

impl EtaCache {
    pub fn new() -> Self {
        Self::default()
    }
}
