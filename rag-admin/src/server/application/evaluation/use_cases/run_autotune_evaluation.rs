use std::sync::Arc;

use crate::server::application::evaluation::ports::EvaluationResultStore;
use crate::server::application::evaluation::progress::EvaluationProgress;
use crate::server::application::evaluation::use_cases::run_evaluation::{
    now_rfc3339, score_prepared_variant_for_indices, RunEvaluationUseCase,
};
use crate::server::application::AppError;
use crate::shared::{
    evaluation_score, ChunkingConfig, ChunkingVariant, EvaluationAutotuneRequest,
    EvaluationAutotuneSummary, EvaluationResultSplit, EvaluationRunOptions, EvaluationRunResult,
};

pub struct RunAutotuneEvaluationUseCase {
    run_evaluation: RunEvaluationUseCase,
    evaluation_result_store: Arc<dyn EvaluationResultStore>,
}

#[derive(Clone)]
struct AutotuneCandidate {
    variant: ChunkingVariant,
    options: EvaluationRunOptions,
    tuning_score: f32,
}

impl RunAutotuneEvaluationUseCase {
    pub fn new(
        run_evaluation: RunEvaluationUseCase,
        evaluation_result_store: Arc<dyn EvaluationResultStore>,
    ) -> Self {
        Self {
            run_evaluation,
            evaluation_result_store,
        }
    }

    pub async fn execute(
        &self,
        slug: &str,
        request: EvaluationAutotuneRequest,
        progress: Arc<dyn EvaluationProgress>,
    ) -> Result<(), AppError> {
        let variants = autotune_variants(&request.current_config);
        let option_sets = autotune_option_sets(&request);
        let candidate_count = variants.len() * option_sets.len();

        progress
            .info(format!(
                "INIT_PROCESS: autotuning post '{slug}' across {} chunker(s), {} option set(s), {} candidate(s)...",
                variants.len(),
                option_sets.len(),
                candidate_count
            ))
            .await;

        let context = self
            .run_evaluation
            .prepare_evaluation_context(slug, Some(&progress))
            .await?;
        let (tuning_indices, holdout_indices) = tuning_holdout_indices(
            context.dataset.questions.len(),
            context.post.version().as_str(),
        );

        if tuning_indices.is_empty() || holdout_indices.is_empty() {
            return Err(AppError::Validation(
                "autotune requires at least two evaluation questions".into(),
            ));
        }

        progress
            .info(format!(
                "AUTOTUNE_SPLIT: tuning={} holdout={} selection uses tuning only",
                tuning_indices.len(),
                holdout_indices.len()
            ))
            .await;

        let mut tuning_results = Vec::with_capacity(candidate_count);
        let mut best_candidate: Option<AutotuneCandidate> = None;
        let mut evaluated = 0usize;

        for variant in variants {
            let mut prepared_with_glossary = None;
            let mut prepared_without_glossary = None;

            for options in &option_sets {
                evaluated += 1;
                progress
                    .info(format!(
                        "AUTOTUNE_CANDIDATE: {}/{} {} TOP_K={} MIN_SCORE={} GLOSSARY={}",
                        evaluated,
                        candidate_count,
                        variant.label,
                        options.top_k,
                        options.min_score_milli,
                        options.include_glossary
                    ))
                    .await;

                let prepared = if options.include_glossary {
                    if prepared_with_glossary.is_none() {
                        prepared_with_glossary = Some(
                            self.run_evaluation
                                .prepare_evaluation_variant(
                                    &context,
                                    variant.clone(),
                                    options,
                                    Some(&progress),
                                )
                                .await?,
                        );
                    }
                    prepared_with_glossary
                        .as_ref()
                        .expect("prepared variant should exist")
                } else {
                    if prepared_without_glossary.is_none() {
                        prepared_without_glossary = Some(
                            self.run_evaluation
                                .prepare_evaluation_variant(
                                    &context,
                                    variant.clone(),
                                    options,
                                    Some(&progress),
                                )
                                .await?,
                        );
                    }
                    prepared_without_glossary
                        .as_ref()
                        .expect("prepared variant should exist")
                };

                let mut result = score_prepared_variant_for_indices(
                    prepared,
                    &context,
                    options,
                    &tuning_indices,
                );
                result.split = EvaluationResultSplit::Tuning;
                let score = evaluation_score(&result.metrics);
                if best_candidate
                    .as_ref()
                    .map(|best| score > best.tuning_score)
                    .unwrap_or(true)
                {
                    best_candidate = Some(AutotuneCandidate {
                        variant: result.variant.clone(),
                        options: options.clone(),
                        tuning_score: score,
                    });
                }
                tuning_results.push(result);
            }
        }

        let Some(best_candidate) = best_candidate else {
            return Err(AppError::Validation(
                "autotune produced no candidate results".into(),
            ));
        };

        progress
            .info(format!(
                "AUTOTUNE_SELECTED: {} TOP_K={} MIN_SCORE={} GLOSSARY={} TUNING_SCORE={:.1}%",
                best_candidate.variant.label,
                best_candidate.options.top_k,
                best_candidate.options.min_score_milli,
                best_candidate.options.include_glossary,
                best_candidate.tuning_score * 100.0
            ))
            .await;

        let prepared = self
            .run_evaluation
            .prepare_evaluation_variant(
                &context,
                best_candidate.variant.clone(),
                &best_candidate.options,
                Some(&progress),
            )
            .await?;
        let mut holdout_result = score_prepared_variant_for_indices(
            &prepared,
            &context,
            &best_candidate.options,
            &holdout_indices,
        );
        holdout_result.split = EvaluationResultSplit::Holdout;
        holdout_result.selected = true;
        let holdout_score = evaluation_score(&holdout_result.metrics);

        progress
            .info(format!(
                "AUTOTUNE_HOLDOUT: SCORE={:.1}% RECALL={:.1}% PRECISION={:.1}%",
                holdout_score * 100.0,
                holdout_result.metrics.recall_mean * 100.0,
                holdout_result.metrics.precision_mean * 100.0
            ))
            .await;

        let mut variants = Vec::with_capacity(tuning_results.len() + 1);
        variants.push(holdout_result);
        variants.extend(tuning_results);

        let result = EvaluationRunResult::new(
            context.post.slug().to_string(),
            context.post.version().as_str().to_string(),
            now_rfc3339(),
            best_candidate.options.clone(),
            Some(EvaluationAutotuneSummary {
                tuning_question_count: tuning_indices.len() as u32,
                holdout_question_count: holdout_indices.len() as u32,
                candidate_count: candidate_count as u32,
                selected_label: best_candidate.variant.label.clone(),
                selected_options: best_candidate.options.clone(),
                selected_config: best_candidate.variant.config,
                tuning_score: best_candidate.tuning_score,
                holdout_score,
            }),
            variants,
        );
        self.evaluation_result_store.store(&result).await?;

        progress
            .success(format!(
                "autotune complete and saved. selected {} with holdout score {:.1}%",
                best_candidate.variant.label,
                holdout_score * 100.0
            ))
            .await;
        Ok(())
    }
}

fn autotune_variants(current_config: &ChunkingConfig) -> Vec<ChunkingVariant> {
    ChunkingConfig::sweep_configs(current_config)
        .into_iter()
        .map(|config| ChunkingVariant {
            label: config.display_label(),
            config: config.clone(),
        })
        .collect()
}

fn autotune_option_sets(request: &EvaluationAutotuneRequest) -> Vec<EvaluationRunOptions> {
    let mut option_sets = Vec::new();
    for top_k in unique_u32_values(&request.top_k_values) {
        for min_score_milli in unique_u32_values(&request.min_score_milli_values) {
            for include_glossary in unique_bool_values(&request.include_glossary_values) {
                option_sets.push(EvaluationRunOptions {
                    top_k,
                    min_score_milli,
                    include_glossary,
                });
            }
        }
    }
    option_sets
}

fn unique_u32_values(values: &[u32]) -> Vec<u32> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(value) {
            unique.push(*value);
        }
    }
    unique
}

fn unique_bool_values(values: &[bool]) -> Vec<bool> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(value) {
            unique.push(*value);
        }
    }
    unique
}

fn tuning_holdout_indices(question_count: usize, seed: &str) -> (Vec<usize>, Vec<usize>) {
    if question_count < 2 {
        return ((0..question_count).collect(), Vec::new());
    }

    let holdout_count = ((question_count as f32 * 0.25).ceil() as usize)
        .max(1)
        .min(question_count - 1);
    let mut ranked = (0..question_count)
        .map(|index| (stable_split_score(seed, index), index))
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(score, _)| *score);

    let mut holdout = ranked
        .iter()
        .take(holdout_count)
        .map(|(_, index)| *index)
        .collect::<Vec<_>>();
    let mut tuning = ranked
        .iter()
        .skip(holdout_count)
        .map(|(_, index)| *index)
        .collect::<Vec<_>>();
    holdout.sort_unstable();
    tuning.sort_unstable();

    (tuning, holdout)
}

fn stable_split_score(seed: &str, index: usize) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in seed.bytes().chain(index.to_le_bytes()) {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_holdout_split_is_deterministic_and_non_overlapping() {
        let (first_tuning, first_holdout) = tuning_holdout_indices(10, "post-version");
        let (second_tuning, second_holdout) = tuning_holdout_indices(10, "post-version");

        assert_eq!(first_tuning, second_tuning);
        assert_eq!(first_holdout, second_holdout);
        assert_eq!(first_tuning.len(), 7);
        assert_eq!(first_holdout.len(), 3);
        assert!(first_tuning
            .iter()
            .all(|index| !first_holdout.contains(index)));
    }

    #[test]
    fn autotune_option_sets_deduplicate_grid_values() {
        let request = EvaluationAutotuneRequest {
            current_config: ChunkingConfig::default(),
            top_k_values: vec![3, 3, 5],
            min_score_milli_values: vec![0, 700, 700],
            include_glossary_values: vec![true, false, true],
        };

        let option_sets = autotune_option_sets(&request);

        assert_eq!(option_sets.len(), 8);
        assert!(option_sets.contains(&EvaluationRunOptions {
            top_k: 3,
            min_score_milli: 700,
            include_glossary: false,
        }));
    }
}
