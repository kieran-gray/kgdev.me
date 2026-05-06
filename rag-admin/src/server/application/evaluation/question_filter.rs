use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::retrieval::cosine_similarity;
use crate::server::application::AppError;
use crate::shared::{EmbeddingModel, EvaluationQuestion};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QuestionFilterStats {
    pub low_excerpt_similarity: u32,
    pub duplicate: u32,
}

pub async fn filter_generated_questions(
    embedding_service: &EmbeddingService,
    model: &EmbeddingModel,
    questions: Vec<EvaluationQuestion>,
    excerpt_similarity_threshold: f32,
    duplicate_similarity_threshold: f32,
) -> Result<(Vec<EvaluationQuestion>, QuestionFilterStats), AppError> {
    if questions.is_empty() {
        return Ok((questions, QuestionFilterStats::default()));
    }

    let question_texts: Vec<String> = questions.iter().map(|q| q.question.clone()).collect();
    let question_embeddings = embedding_service
        .embed_batch(model, &question_texts)
        .await?;

    let mut reference_texts = Vec::new();
    let mut reference_question_indexes = Vec::new();
    for (question_index, question) in questions.iter().enumerate() {
        for reference in &question.references {
            reference_texts.push(reference.content.clone());
            reference_question_indexes.push(question_index);
        }
    }

    let reference_embeddings = if reference_texts.is_empty() {
        Vec::new()
    } else {
        embedding_service
            .embed_batch(model, &reference_texts)
            .await?
    };

    Ok(filter_questions_by_embeddings(
        questions,
        &question_embeddings,
        &reference_embeddings,
        &reference_question_indexes,
        excerpt_similarity_threshold,
        duplicate_similarity_threshold,
    ))
}

fn filter_questions_by_embeddings(
    questions: Vec<EvaluationQuestion>,
    question_embeddings: &[Vec<f32>],
    reference_embeddings: &[Vec<f32>],
    reference_question_indexes: &[usize],
    excerpt_similarity_threshold: f32,
    duplicate_similarity_threshold: f32,
) -> (Vec<EvaluationQuestion>, QuestionFilterStats) {
    let mut references_by_question = vec![Vec::new(); questions.len()];
    for (reference_index, question_index) in reference_question_indexes.iter().copied().enumerate()
    {
        if let Some(reference_indexes) = references_by_question.get_mut(question_index) {
            reference_indexes.push(reference_index);
        }
    }

    let mut kept = Vec::new();
    let mut kept_embedding_indexes: Vec<usize> = Vec::new();
    let mut stats = QuestionFilterStats::default();

    for (question_index, question) in questions.into_iter().enumerate() {
        let Some(question_embedding) = question_embeddings.get(question_index) else {
            stats.low_excerpt_similarity += 1;
            continue;
        };
        let reference_indexes = &references_by_question[question_index];
        if reference_indexes.is_empty() {
            stats.low_excerpt_similarity += 1;
            continue;
        }

        let min_reference_similarity = reference_indexes
            .iter()
            .filter_map(|i| reference_embeddings.get(*i))
            .map(|reference_embedding| cosine_similarity(question_embedding, reference_embedding))
            .fold(f32::INFINITY, f32::min);
        if !min_reference_similarity.is_finite()
            || min_reference_similarity < excerpt_similarity_threshold
        {
            stats.low_excerpt_similarity += 1;
            continue;
        }

        let is_duplicate = kept_embedding_indexes.iter().any(|kept_index| {
            question_embeddings
                .get(*kept_index)
                .map(|kept_embedding| {
                    cosine_similarity(question_embedding, kept_embedding)
                        >= duplicate_similarity_threshold
                })
                .unwrap_or(false)
        });
        if is_duplicate {
            stats.duplicate += 1;
            continue;
        }

        kept_embedding_indexes.push(question_index);
        kept.push(question);
    }

    (kept, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::EvaluationReference;

    fn question(text: &str, reference: &str) -> EvaluationQuestion {
        EvaluationQuestion {
            question: text.into(),
            references: vec![EvaluationReference {
                content: reference.into(),
                char_start: 0,
                char_end: reference.chars().count() as u32,
                embedding: None,
            }],
            embedding: None,
        }
    }

    #[test]
    fn filters_questions_with_low_reference_similarity() {
        let questions = vec![
            question("aligned?", "aligned reference"),
            question("unrelated?", "unrelated reference"),
        ];
        let question_embeddings = vec![vec![1.0, 0.0], vec![1.0, 0.0]];
        let reference_embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let reference_question_indexes = vec![0, 1];

        let (kept, stats) = filter_questions_by_embeddings(
            questions,
            &question_embeddings,
            &reference_embeddings,
            &reference_question_indexes,
            0.5,
            0.95,
        );

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].question, "aligned?");
        assert_eq!(stats.low_excerpt_similarity, 1);
        assert_eq!(stats.duplicate, 0);
    }

    #[test]
    fn filters_duplicate_questions_after_reference_similarity() {
        let questions = vec![
            question("first?", "first reference"),
            question("duplicate?", "duplicate reference"),
            question("different?", "different reference"),
        ];
        let question_embeddings = vec![vec![1.0, 0.0], vec![0.99, 0.01], vec![0.0, 1.0]];
        let reference_embeddings = vec![vec![1.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]];
        let reference_question_indexes = vec![0, 1, 2];

        let (kept, stats) = filter_questions_by_embeddings(
            questions,
            &question_embeddings,
            &reference_embeddings,
            &reference_question_indexes,
            0.5,
            0.95,
        );

        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].question, "first?");
        assert_eq!(kept[1].question, "different?");
        assert_eq!(stats.low_excerpt_similarity, 0);
        assert_eq!(stats.duplicate, 1);
    }
}
