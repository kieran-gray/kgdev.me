use crate::server::application::evaluation::retrieval::{retrieve_chunks, EvalChunk};
use crate::shared::{
    ChunkingVariant, EvaluationMetrics, EvaluationQuestion, EvaluationQuestionResult,
    EvaluationReferenceResult, EvaluationResultSplit, EvaluationRunOptions,
    EvaluationVariantResult,
};

pub fn evaluate_variant(
    variant: ChunkingVariant,
    questions: &[EvaluationQuestion],
    chunks: &[EvalChunk],
    chunk_embeddings: &[Vec<f32>],
    question_embeddings: &[Vec<f32>],
    options: &EvaluationRunOptions,
) -> EvaluationVariantResult {
    let mut question_results = Vec::with_capacity(questions.len());
    let mut recall_scores = Vec::with_capacity(questions.len());
    let mut precision_scores = Vec::with_capacity(questions.len());
    let mut iou_scores = Vec::with_capacity(questions.len());
    let mut omega_scores = Vec::with_capacity(questions.len());
    let mut retrieved_tokens = Vec::with_capacity(questions.len());

    for (question, question_embedding) in questions.iter().zip(question_embeddings) {
        let retrieved = retrieve_chunks(question_embedding, chunks, chunk_embeddings, options);
        let retrieved_refs: Vec<&EvalChunk> =
            retrieved.iter().map(|r| &chunks[r.chunk_index]).collect();
        let score = score_question(question, &retrieved_refs);
        let omega = precision_omega(question, chunks);

        retrieved_tokens.push(
            retrieved_refs
                .iter()
                .map(|c| c.retrieved_tokens())
                .sum::<u32>(),
        );
        recall_scores.push(score.recall);
        precision_scores.push(score.precision);
        iou_scores.push(score.iou);
        omega_scores.push(omega);
        question_results.push(EvaluationQuestionResult {
            question: question.question.clone(),
            recall: score.recall,
            precision: score.precision,
            iou: score.iou,
            retrieved_chunk_ids: retrieved_refs.iter().map(|c| c.chunk_id).collect(),
            missed_reference_count: score.missed_reference_count,
            reference_results: score.reference_results,
        });
    }

    let chunk_count = chunks.len() as u32;
    let average_chunk_tokens = if chunks.is_empty() {
        0
    } else {
        chunks.iter().map(|c| c.token_count).sum::<u32>() / chunk_count
    };
    let average_retrieved_tokens = mean_u32(&retrieved_tokens);

    EvaluationVariantResult {
        variant,
        options: options.clone(),
        split: EvaluationResultSplit::Full,
        selected: false,
        metrics: EvaluationMetrics {
            recall_mean: mean(&recall_scores),
            recall_std: std_dev(&recall_scores),
            precision_mean: mean(&precision_scores),
            precision_std: std_dev(&precision_scores),
            iou_mean: mean(&iou_scores),
            iou_std: std_dev(&iou_scores),
            precision_omega_mean: mean(&omega_scores),
            precision_omega_std: std_dev(&omega_scores),
        },
        chunk_count,
        average_chunk_tokens,
        average_retrieved_tokens,
        question_results,
    }
}

#[derive(Debug, Clone)]
struct QuestionScore {
    recall: f32,
    precision: f32,
    iou: f32,
    missed_reference_count: u32,
    reference_results: Vec<EvaluationReferenceResult>,
}

fn score_question(question: &EvaluationQuestion, retrieved_chunks: &[&EvalChunk]) -> QuestionScore {
    let reference_ranges: Vec<Range> = question
        .references
        .iter()
        .map(|r| Range::new(r.char_start, r.char_end))
        .filter(|r| r.len() > 0)
        .collect();
    let relevant_len = sum_ranges(&reference_ranges);
    if relevant_len == 0 {
        return QuestionScore {
            recall: 0.0,
            precision: 0.0,
            iou: 0.0,
            missed_reference_count: question.references.len() as u32,
            reference_results: Vec::new(),
        };
    }

    let mut intersection_ranges = Vec::new();
    let mut missed = reference_ranges.clone();
    for chunk in retrieved_chunks.iter().filter(|c| c.body_chunk) {
        let chunk_range = Range::new(chunk.char_start, chunk.char_end);
        for reference in &reference_ranges {
            if let Some(intersection) = chunk_range.intersect(*reference) {
                intersection_ranges.push(intersection);
                missed = subtract_range(&missed, intersection);
            }
        }
    }

    let numerator = sum_ranges(&union_ranges(intersection_ranges));
    let retrieved_len: u32 = retrieved_chunks.iter().map(|c| c.retrieved_len()).sum();
    let unused_relevant_len = sum_ranges(&missed);

    let precision = if retrieved_len == 0 {
        0.0
    } else {
        numerator as f32 / retrieved_len as f32
    };
    let recall = numerator as f32 / relevant_len as f32;
    let iou_denominator = retrieved_len + unused_relevant_len;
    let iou = if iou_denominator == 0 {
        0.0
    } else {
        numerator as f32 / iou_denominator as f32
    };

    QuestionScore {
        recall,
        precision,
        iou,
        missed_reference_count: missed_reference_count(&reference_ranges, &missed),
        reference_results: reference_results(question, retrieved_chunks),
    }
}

fn reference_results(
    question: &EvaluationQuestion,
    retrieved_chunks: &[&EvalChunk],
) -> Vec<EvaluationReferenceResult> {
    question
        .references
        .iter()
        .filter_map(|reference| {
            let reference_range = Range::new(reference.char_start, reference.char_end);
            let total_chars = reference_range.len();
            if total_chars == 0 {
                return None;
            }

            let intersections = retrieved_chunks
                .iter()
                .filter(|chunk| chunk.body_chunk)
                .filter_map(|chunk| {
                    Range::new(chunk.char_start, chunk.char_end).intersect(reference_range)
                })
                .collect::<Vec<_>>();
            let covered_chars = sum_ranges(&union_ranges(intersections));

            Some(EvaluationReferenceResult {
                content: reference.content.clone(),
                char_start: reference.char_start,
                char_end: reference.char_end,
                covered_chars,
                total_chars,
                recall: covered_chars as f32 / total_chars as f32,
            })
        })
        .collect()
}

fn precision_omega(question: &EvaluationQuestion, chunks: &[EvalChunk]) -> f32 {
    let reference_ranges: Vec<Range> = question
        .references
        .iter()
        .map(|r| Range::new(r.char_start, r.char_end))
        .filter(|r| r.len() > 0)
        .collect();
    if reference_ranges.is_empty() {
        return 0.0;
    }

    let touching: Vec<&EvalChunk> = chunks
        .iter()
        .filter(|chunk| {
            chunk.body_chunk
                && reference_ranges.iter().any(|reference| {
                    Range::new(chunk.char_start, chunk.char_end)
                        .intersect(*reference)
                        .is_some()
                })
        })
        .collect();
    score_question(question, &touching).precision
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Range {
    start: u32,
    end: u32,
}

impl Range {
    fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    fn intersect(self, other: Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        (start < end).then_some(Self { start, end })
    }
}

fn union_ranges(mut ranges: Vec<Range>) -> Vec<Range> {
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_by_key(|r| r.start);
    let mut merged = vec![ranges[0]];
    for current in ranges.into_iter().skip(1) {
        let last = merged.last_mut().expect("merged range");
        if current.start <= last.end {
            last.end = last.end.max(current.end);
        } else {
            merged.push(current);
        }
    }
    merged
}

fn subtract_range(ranges: &[Range], target: Range) -> Vec<Range> {
    let mut result = Vec::new();
    for range in ranges {
        if range.end <= target.start || range.start >= target.end {
            result.push(*range);
        } else {
            if range.start < target.start {
                result.push(Range::new(range.start, target.start));
            }
            if range.end > target.end {
                result.push(Range::new(target.end, range.end));
            }
        }
    }
    result
}

fn sum_ranges(ranges: &[Range]) -> u32 {
    ranges.iter().map(|r| r.len()).sum()
}

fn missed_reference_count(reference_ranges: &[Range], missed_ranges: &[Range]) -> u32 {
    reference_ranges
        .iter()
        .filter(|reference| {
            missed_ranges
                .iter()
                .any(|missed| missed.intersect(**reference).is_some())
        })
        .count() as u32
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

fn std_dev(values: &[f32]) -> f32 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f32>() / values.len() as f32;
    variance.sqrt()
}

fn mean_u32(values: &[u32]) -> u32 {
    if values.is_empty() {
        0
    } else {
        values.iter().sum::<u32>() / values.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{ChunkingConfig, EvaluationReference};

    const EPS: f32 = 1e-4;

    fn assert_close(actual: f32, expected: f32, label: &str) {
        assert!(
            (actual - expected).abs() < EPS,
            "{label}: expected {expected}, got {actual}"
        );
    }

    fn fixture_variant() -> ChunkingVariant {
        ChunkingVariant {
            label: "fixture".into(),
            config: ChunkingConfig::default(),
        }
    }

    // Three body chunks spanning chars 0..200, 10 tokens each.
    fn fixture_chunks() -> Vec<EvalChunk> {
        vec![
            EvalChunk {
                chunk_id: 0,
                text: "c0".into(),
                token_count: 10,
                char_start: 0,
                char_end: 50,
                body_chunk: true,
            },
            EvalChunk {
                chunk_id: 1,
                text: "c1".into(),
                token_count: 10,
                char_start: 50,
                char_end: 120,
                body_chunk: true,
            },
            EvalChunk {
                chunk_id: 2,
                text: "c2".into(),
                token_count: 10,
                char_start: 120,
                char_end: 200,
                body_chunk: true,
            },
        ]
    }

    // Embeddings chosen so Q1 prefers C0 then C1, and Q2 prefers C2 then C1.
    fn fixture_chunk_embeddings() -> Vec<Vec<f32>> {
        vec![
            vec![0.9, 0.1, 0.0, 0.0],
            vec![0.5, 0.5, 0.0, 0.0],
            vec![0.1, 0.9, 0.0, 0.0],
        ]
    }

    // Q1 reference 30..80 straddles C0 and C1; Q2 reference 100..150 straddles C1 and C2.
    fn fixture_questions() -> Vec<EvaluationQuestion> {
        vec![
            EvaluationQuestion {
                question: "q1".into(),
                references: vec![EvaluationReference {
                    content: "ref1".into(),
                    char_start: 30,
                    char_end: 80,
                    embedding: None,
                }],
                embedding: None,
            },
            EvaluationQuestion {
                question: "q2".into(),
                references: vec![EvaluationReference {
                    content: "ref2".into(),
                    char_start: 100,
                    char_end: 150,
                    embedding: None,
                }],
                embedding: None,
            },
        ]
    }

    fn fixture_question_embeddings() -> Vec<Vec<f32>> {
        vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0]]
    }

    fn options(top_k: u32) -> EvaluationRunOptions {
        EvaluationRunOptions {
            top_k,
            min_score_milli: 0,
            include_glossary: false,
        }
    }

    #[test]
    fn evaluate_variant_perfect_recall_with_top_k_2() {
        // Q1 retrieves [C0, C1], fully covering ref 30..80 across both chunks.
        // Q2 retrieves [C2, C1], fully covering ref 100..150.
        let result = evaluate_variant(
            fixture_variant(),
            &fixture_questions(),
            &fixture_chunks(),
            &fixture_chunk_embeddings(),
            &fixture_question_embeddings(),
            &options(2),
        );

        assert_eq!(result.chunk_count, 3);
        assert_eq!(result.average_chunk_tokens, 10);
        assert_eq!(result.average_retrieved_tokens, 20);

        // Hand-computed:
        //   Q1: numerator=50, retrieved=120, missed=0  → recall=1, precision=50/120, iou=50/120
        //   Q2: numerator=50, retrieved=150, missed=0  → recall=1, precision=50/150, iou=50/150
        //   precision_omega is the same as precision when retrieved == touching.
        let q1_p = 50.0 / 120.0;
        let q2_p = 50.0 / 150.0;
        let mean_p = (q1_p + q2_p) / 2.0;
        assert_close(result.metrics.recall_mean, 1.0, "recall_mean");
        assert_close(result.metrics.recall_std, 0.0, "recall_std");
        assert_close(result.metrics.precision_mean, mean_p, "precision_mean");
        assert_close(result.metrics.iou_mean, mean_p, "iou_mean");
        assert_close(
            result.metrics.precision_omega_mean,
            mean_p,
            "precision_omega_mean",
        );

        let q1 = &result.question_results[0];
        assert_close(q1.recall, 1.0, "q1.recall");
        assert_close(q1.precision, q1_p, "q1.precision");
        assert_close(q1.iou, q1_p, "q1.iou");
        assert_eq!(q1.missed_reference_count, 0);
        assert_eq!(q1.retrieved_chunk_ids, vec![0, 1]);
        assert_eq!(q1.reference_results.len(), 1);
        assert_eq!(q1.reference_results[0].covered_chars, 50);
        assert_eq!(q1.reference_results[0].total_chars, 50);
        assert_close(q1.reference_results[0].recall, 1.0, "q1 ref recall");

        let q2 = &result.question_results[1];
        assert_close(q2.recall, 1.0, "q2.recall");
        assert_close(q2.precision, q2_p, "q2.precision");
        assert_close(q2.iou, q2_p, "q2.iou");
        assert_eq!(q2.missed_reference_count, 0);
        assert_eq!(q2.retrieved_chunk_ids, vec![2, 1]);
    }

    #[test]
    fn evaluate_variant_partial_recall_with_top_k_1() {
        // Q1 retrieves only [C0], missing the 50..80 portion of the reference.
        // Q2 retrieves only [C2], missing the 100..120 portion.
        let result = evaluate_variant(
            fixture_variant(),
            &fixture_questions(),
            &fixture_chunks(),
            &fixture_chunk_embeddings(),
            &fixture_question_embeddings(),
            &options(1),
        );

        // Q1: numerator=20, retrieved=50, missed=30  → recall=0.4, precision=0.4, iou=20/80=0.25
        // Q2: numerator=30, retrieved=80, missed=20  → recall=0.6, precision=0.375, iou=30/100=0.30
        let q1 = &result.question_results[0];
        assert_close(q1.recall, 0.4, "q1.recall");
        assert_close(q1.precision, 0.4, "q1.precision");
        assert_close(q1.iou, 0.25, "q1.iou");
        assert_eq!(q1.missed_reference_count, 1);
        assert_eq!(q1.retrieved_chunk_ids, vec![0]);
        assert_eq!(q1.reference_results[0].covered_chars, 20);
        assert_eq!(q1.reference_results[0].total_chars, 50);

        let q2 = &result.question_results[1];
        assert_close(q2.recall, 0.6, "q2.recall");
        assert_close(q2.precision, 0.375, "q2.precision");
        assert_close(q2.iou, 0.30, "q2.iou");
        assert_eq!(q2.missed_reference_count, 1);
        assert_eq!(q2.retrieved_chunk_ids, vec![2]);

        // precision_omega depends only on the chunks that touch the reference, not on top_k,
        // so it stays at the same value as the top_k=2 case.
        let omega = (50.0 / 120.0 + 50.0 / 150.0) / 2.0;
        assert_close(
            result.metrics.precision_omega_mean,
            omega,
            "precision_omega_mean",
        );
        assert_close(result.metrics.recall_mean, 0.5, "recall_mean");
        assert_close(
            result.metrics.precision_mean,
            (0.4 + 0.375) / 2.0,
            "precision_mean",
        );
        assert_close(
            result.metrics.iou_mean,
            (0.25 + 0.30) / 2.0,
            "iou_mean",
        );

        // Each retrieval is one body chunk of 10 tokens.
        assert_eq!(result.average_retrieved_tokens, 10);
    }

    #[test]
    fn evaluate_variant_glossary_chunks_excluded_from_span_math() {
        // A glossary chunk should never count toward span overlap, even if retrieved.
        let mut chunks = fixture_chunks();
        chunks.push(EvalChunk {
            chunk_id: 99,
            text: "glossary entry".into(),
            token_count: 5,
            char_start: 0,
            char_end: 200,
            body_chunk: false,
        });
        let mut chunk_embeddings = fixture_chunk_embeddings();
        // Make the glossary chunk score highest for Q1 so it's retrieved first.
        chunk_embeddings.push(vec![1.0, 0.0, 0.0, 0.0]);

        let result = evaluate_variant(
            fixture_variant(),
            &fixture_questions(),
            &chunks,
            &chunk_embeddings,
            &fixture_question_embeddings(),
            &options(1),
        );

        // For Q1, only the glossary chunk is retrieved (top_k=1); it's body_chunk=false so it
        // contributes zero overlap, but its `retrieved_len` (text length) inflates the denominator.
        let q1 = &result.question_results[0];
        assert_eq!(q1.retrieved_chunk_ids, vec![99]);
        assert_close(q1.recall, 0.0, "q1.recall (glossary-only)");
        assert_close(q1.precision, 0.0, "q1.precision (glossary-only)");
        assert_eq!(q1.missed_reference_count, 1);
    }

    #[test]
    fn evaluate_variant_handles_question_with_no_references() {
        let questions = vec![EvaluationQuestion {
            question: "q-empty".into(),
            references: Vec::new(),
            embedding: None,
        }];
        let question_embeddings = vec![vec![1.0, 0.0, 0.0, 0.0]];

        let result = evaluate_variant(
            fixture_variant(),
            &questions,
            &fixture_chunks(),
            &fixture_chunk_embeddings(),
            &question_embeddings,
            &options(2),
        );

        let q = &result.question_results[0];
        assert_close(q.recall, 0.0, "recall on no-refs question");
        assert_close(q.precision, 0.0, "precision on no-refs question");
        assert_close(q.iou, 0.0, "iou on no-refs question");
        assert_eq!(q.missed_reference_count, 0);
        assert!(q.reference_results.is_empty());
        assert_close(
            result.metrics.precision_omega_mean,
            0.0,
            "precision_omega on no-refs question",
        );
    }
}
