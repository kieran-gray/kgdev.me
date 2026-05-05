use crate::server::application::chunking::ChunkOutput;
use crate::server::domain::Chunk;
use crate::shared::EvaluationRunOptions;

#[derive(Debug, Clone)]
pub struct EvalChunk {
    pub chunk_id: u32,
    pub text: String,
    pub char_start: u32,
    pub char_end: u32,
    pub body_chunk: bool,
}

impl EvalChunk {
    pub fn retrieved_len(&self) -> u32 {
        if self.body_chunk {
            self.char_end.saturating_sub(self.char_start)
        } else {
            self.text.chars().count() as u32
        }
    }
}

impl From<ChunkOutput> for EvalChunk {
    fn from(value: ChunkOutput) -> Self {
        Self {
            chunk_id: value.chunk_id,
            text: value.text,
            char_start: value.char_start,
            char_end: value.char_end,
            body_chunk: true,
        }
    }
}

impl From<Chunk> for EvalChunk {
    fn from(value: Chunk) -> Self {
        Self {
            chunk_id: value.chunk_id,
            text: value.text,
            char_start: value.char_start,
            char_end: value.char_end,
            body_chunk: !value.is_glossary,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetrievedChunk {
    pub chunk_index: usize,
}

pub fn retrieve_chunks(
    question_embedding: &[f32],
    chunks: &[EvalChunk],
    chunk_embeddings: &[Vec<f32>],
    options: &EvaluationRunOptions,
) -> Vec<RetrievedChunk> {
    let mut scored: Vec<(usize, f32)> = chunk_embeddings
        .iter()
        .enumerate()
        .map(|(i, emb)| (i, cosine_similarity(question_embedding, emb)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored
        .into_iter()
        .take(options.top_k as usize)
        .filter(|(_, score)| *score >= options.min_score())
        .filter(|(i, _)| chunks.get(*i).is_some())
        .map(|(chunk_index, _score)| RetrievedChunk { chunk_index })
        .collect()
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
