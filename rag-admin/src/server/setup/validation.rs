use crate::shared::{
    catalog_for_backend, EmbedderBackend, EvaluationGenerationBackend, SettingsDto,
};

pub fn validate_local(s: &SettingsDto) -> Result<(), String> {
    if s.embedding_model.id.is_empty() {
        return Err("embedding model id is empty".into());
    }
    if s.embedding_model.dims == 0 {
        return Err("embedding model dims must be > 0".into());
    }

    let index_dims = s.vector_index.dimensions();
    if index_dims == 0 {
        return Err("vector_index dimensions must be > 0".into());
    }
    if index_dims != s.embedding_model.dims {
        return Err(format!(
            "vector index dimensions ({}) must equal embedding model dims ({})",
            index_dims, s.embedding_model.dims
        ));
    }
    if s.vector_index.name().is_empty() {
        return Err("vector_index name is empty".into());
    }

    let backend = s.embedding_model.backend;
    let catalog = catalog_for_backend(backend);
    if let Some(entry) = catalog.iter().find(|e| e.id == s.embedding_model.id) {
        if matches!(backend, EmbedderBackend::Cloudflare) && entry.dims != s.embedding_model.dims {
            return Err(format!(
                "Cloudflare model '{}' produces {}-dim vectors but settings declare {}",
                s.embedding_model.id, entry.dims, s.embedding_model.dims
            ));
        }
    } else if !is_id_well_formed(backend, &s.embedding_model.id) {
        return Err(format!(
            "model id '{}' does not look valid for the {} backend",
            s.embedding_model.id,
            backend.as_str()
        ));
    }

    if s.evaluation.generation_model.trim().is_empty() {
        return Err("evaluation generation model is empty".into());
    }
    if s.evaluation.question_count == 0 {
        return Err("evaluation question count must be > 0".into());
    }
    if s.evaluation.top_k == 0 {
        return Err("evaluation top_k must be > 0".into());
    }
    if s.evaluation.excerpt_similarity_threshold_milli > 1000 {
        return Err("evaluation excerpt similarity threshold must be <= 1000".into());
    }
    if s.evaluation.duplicate_similarity_threshold_milli > 1000 {
        return Err("evaluation duplicate similarity threshold must be <= 1000".into());
    }
    if s.evaluation.min_score_milli > 1000 {
        return Err("evaluation min score must be <= 1000".into());
    }
    if matches!(
        s.evaluation.generation_backend,
        EvaluationGenerationBackend::Ollama
    ) {
        let url = s.evaluation.ollama_base_url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("evaluation Ollama base URL must start with http:// or https://".into());
        }
    }

    Ok(())
}

fn is_id_well_formed(backend: EmbedderBackend, id: &str) -> bool {
    match backend {
        EmbedderBackend::Cloudflare => id.starts_with("@cf/"),
        EmbedderBackend::Ollama => !id.contains(char::is_whitespace),
    }
}
