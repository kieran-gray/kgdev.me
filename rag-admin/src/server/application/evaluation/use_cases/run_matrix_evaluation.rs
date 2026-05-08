use std::sync::Arc;

use crate::server::application::evaluation::ports::EvaluationResultStore;
use crate::server::application::evaluation::progress::EvaluationProgress;
use crate::server::application::evaluation::use_cases::run_evaluation::{
    now_rfc3339, score_prepared_variant, RunEvaluationUseCase,
};
use crate::server::application::AppError;
use crate::shared::{ChunkingVariant, EvaluationRunOptions, EvaluationRunResult};

pub struct RunMatrixEvaluationUseCase {
    run_evaluation: RunEvaluationUseCase,
    evaluation_result_store: Arc<dyn EvaluationResultStore>,
}

impl RunMatrixEvaluationUseCase {
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
        variant: ChunkingVariant,
        option_sets: Vec<EvaluationRunOptions>,
        progress: Arc<dyn EvaluationProgress>,
    ) -> Result<(), AppError> {
        let total = option_sets.len();
        let first_options = option_sets.first().cloned().ok_or_else(|| {
            AppError::Validation("at least one evaluation option set is required".into())
        })?;

        progress
            .info(format!(
                "INIT_PROCESS: starting matrix evaluation for post '{slug}'..."
            ))
            .await;

        let context = self
            .run_evaluation
            .prepare_evaluation_context(slug, Some(&progress))
            .await?;
        let mut prepared_with_glossary = None;
        let mut prepared_without_glossary = None;
        let mut matrix_variants = Vec::with_capacity(total);

        for (index, options) in option_sets.iter().enumerate() {
            progress
                .info(format!(
                    "PARAM_SWEEP: scoring {}/{} for '{}' with TOP_K={} MIN_SCORE={}",
                    index + 1,
                    total,
                    variant.label,
                    options.top_k,
                    options.min_score_milli
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

            let mut variant_result = score_prepared_variant(prepared, &context, options);
            variant_result.variant.label =
                matrix_variant_label(&variant_result.variant.label, options);
            matrix_variants.push(variant_result);
        }

        let result = EvaluationRunResult::new(
            context.post.slug().to_string(),
            context.post.version().as_str().to_string(),
            now_rfc3339(),
            first_options,
            None,
            matrix_variants,
        );
        self.evaluation_result_store.store(&result).await?;

        progress
            .success(format!(
                "matrix evaluation complete and saved as one run with {} result(s).",
                result.variants.len()
            ))
            .await;
        Ok(())
    }
}

fn matrix_variant_label(label: &str, _options: &EvaluationRunOptions) -> String {
    label.to_string()
}
