use crate::shared::{
    AiProviderDto, EmbeddingModelDto, GenerationModelDto, VectorIndexDto, VectorStoreProviderDto,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddDialog {
    Provider,
    EmbeddingModel,
    GenerationModel,
    VectorIndex,
}

#[derive(Debug, Clone)]
pub enum EditDialog {
    AiProvider(AiProviderDto),
    VectorStoreProvider(VectorStoreProviderDto),
    EmbeddingModel(EmbeddingModelDto),
    GenerationModel(GenerationModelDto),
    VectorIndex(VectorIndexDto),
}

#[derive(Debug, Clone)]
pub enum DeleteDialog {
    AiProvider(AiProviderDto),
    VectorStoreProvider(VectorStoreProviderDto),
    EmbeddingModel(EmbeddingModelDto),
    GenerationModel(GenerationModelDto),
    VectorIndex(VectorIndexDto),
}

pub fn delete_dialog_label(dialog: Option<DeleteDialog>) -> String {
    match dialog {
        Some(DeleteDialog::AiProvider(p)) => {
            format!(
                "Delete AI provider '{}' and clear it from the registry.",
                p.name
            )
        }
        Some(DeleteDialog::VectorStoreProvider(p)) => {
            format!(
                "Delete vector store provider '{}'. All indexes referencing it must be removed first.",
                p.name
            )
        }
        Some(DeleteDialog::EmbeddingModel(m)) => format!("Delete embedding model '{}'.", m.model),
        Some(DeleteDialog::GenerationModel(m)) => {
            format!("Delete generation model '{}'.", m.model)
        }
        Some(DeleteDialog::VectorIndex(i)) => format!("Delete vector index '{}'.", i.name),
        None => String::new(),
    }
}
