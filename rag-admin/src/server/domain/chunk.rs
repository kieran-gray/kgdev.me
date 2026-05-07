#[derive(Debug, Clone)]
pub struct Chunk {
    pub chunk_id: u32,
    pub heading: String,
    pub text: String,
    pub char_start: u32,
    pub char_end: u32,
    pub sources: Vec<super::GlossarySource>,
    pub is_glossary: bool,
}
