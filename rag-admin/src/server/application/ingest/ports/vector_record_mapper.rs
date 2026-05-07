use crate::server::domain::{Chunk, Post, VectorRecord};

pub trait VectorRecordMapper: Send + Sync {
    fn map(&self, post: &Post, chunk: &Chunk, values: Vec<f32>) -> VectorRecord;
    fn map_id(&self, post: &Post, chunk: &Chunk) -> String;
}
