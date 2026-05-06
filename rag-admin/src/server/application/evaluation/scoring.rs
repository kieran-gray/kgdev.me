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
