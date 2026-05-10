use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::shared::Timestamp;
use crate::server::domain::Aggregate;
use crate::shared::{
    ChunkingVariant, EvaluationAutotuneRequest, EvaluationResultSplit, EvaluationRunOptions,
};

use super::{
    commands::EvaluationRunCommand,
    events::{
        EvaluationRunEvent, RunCompleted, RunFailed, RunRequested, VariantPrepared, VariantScored,
    },
    exceptions::EvaluationRunError,
    scoring_policy::ScoringPolicy,
};

const EVAL_RUN_NAMESPACE: Uuid = uuid::uuid!("b2e4f6a8-c0d2-4e6f-8012-3456789abcde");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvaluationRunStatus {
    Pending,
    Running { variants_completed: u32 },
    Completed,
    Failed { reason: String },
}

impl EvaluationRunStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedVariant {
    pub variant_label: String,
    pub chunk_set_id: Uuid,
    pub embedding_set_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScoredVariantKey {
    pub variant_label: String,
    pub split: EvaluationResultSplit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRun {
    pub run_id: Uuid,
    pub dataset_id: Uuid,
    pub pipeline_configuration_id: Uuid,
    pub document_id: Uuid,
    pub document_version: u32,
    pub variants: Vec<ChunkingVariant>,
    pub options: Vec<EvaluationRunOptions>,
    pub autotune_request: Option<EvaluationAutotuneRequest>,
    pub scoring_policy: ScoringPolicy,
    pub prepared_variants: Vec<PreparedVariant>,
    pub scored_variants: Vec<ScoredVariantKey>,
    pub status: EvaluationRunStatus,
    pub created_at: Timestamp,
}

impl EvaluationRun {
    pub fn compute_id(
        dataset_id: Uuid,
        pipeline_configuration_id: Uuid,
        variants: &[ChunkingVariant],
        options: &[EvaluationRunOptions],
        autotune_request: Option<&EvaluationAutotuneRequest>,
    ) -> Uuid {
        let key = serde_json::to_string(&(
            dataset_id,
            pipeline_configuration_id,
            variants,
            options,
            autotune_request,
        ))
        .unwrap_or_default();
        Uuid::new_v5(&EVAL_RUN_NAMESPACE, key.as_bytes())
    }

    fn from_requested(e: &RunRequested) -> Self {
        Self {
            run_id: e.run_id,
            dataset_id: e.dataset_id,
            pipeline_configuration_id: e.pipeline_configuration_id,
            document_id: e.document_id,
            document_version: e.document_version,
            variants: e.variants.clone(),
            options: e.options.clone(),
            autotune_request: e.autotune_request.clone(),
            scoring_policy: e.scoring_policy,
            prepared_variants: Vec::new(),
            scored_variants: Vec::new(),
            status: EvaluationRunStatus::Pending,
            created_at: e.occurred_at.clone(),
        }
    }

    fn expected_score_count(&self) -> u32 {
        let splits = if self.autotune_request.is_some() {
            2
        } else {
            1
        };
        (self.variants.len() * self.options.len() * splits) as u32
    }

    fn is_variant_prepared(&self, label: &str) -> bool {
        self.prepared_variants
            .iter()
            .any(|p| p.variant_label == label)
    }

    fn is_variant_scored(&self, label: &str, split: &EvaluationResultSplit) -> bool {
        self.scored_variants
            .iter()
            .any(|s| s.variant_label == label && &s.split == split)
    }
}

impl Aggregate for EvaluationRun {
    type Event = EvaluationRunEvent;
    type Command = EvaluationRunCommand;
    type Error = EvaluationRunError;

    fn aggregate_id(&self) -> String {
        self.run_id.to_string()
    }

    fn apply(&mut self, event: &Self::Event) {
        match event {
            Self::Event::RunRequested(_) => {}
            Self::Event::VariantPrepared(e) => {
                if !self.is_variant_prepared(&e.variant_label) {
                    self.prepared_variants.push(PreparedVariant {
                        variant_label: e.variant_label.clone(),
                        chunk_set_id: e.chunk_set_id,
                        embedding_set_id: e.embedding_set_id,
                    });
                }
                self.status = EvaluationRunStatus::Running {
                    variants_completed: 0,
                };
            }
            Self::Event::VariantScored(e) => {
                let key = ScoredVariantKey {
                    variant_label: e.variant_label.clone(),
                    split: e.split,
                };
                if !self.scored_variants.contains(&key) {
                    self.scored_variants.push(key);
                }
                self.status = EvaluationRunStatus::Running {
                    variants_completed: self.scored_variants.len() as u32,
                };
            }
            Self::Event::RunCompleted(_) => {
                self.status = EvaluationRunStatus::Completed;
            }
            Self::Event::RunFailed(e) => {
                self.status = EvaluationRunStatus::Failed {
                    reason: e.reason.clone(),
                };
            }
        }
    }

    fn handle_command(
        state: Option<&Self>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            Self::Command::RequestRun(cmd) => match state {
                None => Ok(vec![Self::Event::RunRequested(RunRequested {
                    run_id: cmd.run_id,
                    dataset_id: cmd.dataset_id,
                    pipeline_configuration_id: cmd.pipeline_configuration_id,
                    document_id: cmd.document_id,
                    document_version: cmd.document_version,
                    variants: cmd.variants,
                    options: cmd.options,
                    autotune_request: cmd.autotune_request,
                    scoring_policy: cmd.scoring_policy,
                    occurred_at: cmd.occurred_at,
                })]),
                Some(run) if run.status.is_terminal() => {
                    if matches!(run.status, EvaluationRunStatus::Completed) {
                        Ok(vec![])
                    } else {
                        Ok(vec![Self::Event::RunRequested(RunRequested {
                            run_id: cmd.run_id,
                            dataset_id: cmd.dataset_id,
                            pipeline_configuration_id: cmd.pipeline_configuration_id,
                            document_id: cmd.document_id,
                            document_version: cmd.document_version,
                            variants: cmd.variants,
                            options: cmd.options,
                            autotune_request: cmd.autotune_request,
                            scoring_policy: cmd.scoring_policy,
                            occurred_at: cmd.occurred_at,
                        })])
                    }
                }
                Some(_) => Err(EvaluationRunError::AlreadyExists),
            },

            Self::Command::MarkVariantPrepared(cmd) => {
                let run = state.ok_or(EvaluationRunError::NotFound)?;
                if run.is_variant_prepared(&cmd.variant_label) {
                    return Ok(vec![]);
                }
                Ok(vec![Self::Event::VariantPrepared(VariantPrepared {
                    run_id: run.run_id,
                    variant_label: cmd.variant_label,
                    chunk_set_id: cmd.chunk_set_id,
                    embedding_set_id: cmd.embedding_set_id,
                    occurred_at: cmd.occurred_at,
                })])
            }

            Self::Command::ScoreVariant(cmd) => {
                let run = state.ok_or(EvaluationRunError::NotFound)?;
                if run.is_variant_scored(&cmd.variant_label, &cmd.split) {
                    return Ok(vec![]);
                }
                Ok(vec![Self::Event::VariantScored(VariantScored {
                    run_id: run.run_id,
                    variant_label: cmd.variant_label,
                    split: cmd.split,
                    metrics: cmd.metrics,
                    retrieval_traces: cmd.retrieval_traces,
                    selected: cmd.selected,
                    occurred_at: cmd.occurred_at,
                })])
            }

            Self::Command::CompleteRun(cmd) => {
                let run = state.ok_or(EvaluationRunError::NotFound)?;
                match &run.status {
                    EvaluationRunStatus::Completed => return Ok(vec![]),
                    EvaluationRunStatus::Failed { .. } => {
                        return Err(EvaluationRunError::AlreadyFailed)
                    }
                    _ => {}
                }
                if run.scored_variants.len() < run.expected_score_count() as usize {
                    return Err(EvaluationRunError::NotAllVariantsScored);
                }
                Ok(vec![Self::Event::RunCompleted(RunCompleted {
                    run_id: run.run_id,
                    occurred_at: cmd.occurred_at,
                })])
            }

            Self::Command::FailRun(cmd) => {
                let run = state.ok_or(EvaluationRunError::NotFound)?;
                match &run.status {
                    EvaluationRunStatus::Completed => {
                        return Err(EvaluationRunError::AlreadyCompleted)
                    }
                    EvaluationRunStatus::Failed { .. } => return Ok(vec![]),
                    _ => {}
                }
                Ok(vec![Self::Event::RunFailed(RunFailed {
                    run_id: run.run_id,
                    reason: cmd.reason,
                    occurred_at: cmd.occurred_at,
                })])
            }
        }
    }

    fn from_events(events: &[Self::Event]) -> Option<Self> {
        let mut state: Option<Self> = None;

        for event in events {
            match (&mut state, event) {
                (None, Self::Event::RunRequested(e)) => {
                    state = Some(Self::from_requested(e));
                }
                (Some(_), Self::Event::RunRequested(e)) => {
                    state = Some(Self::from_requested(e));
                }
                (None, _) => return None,
                (Some(run), event) => run.apply(event),
            }
        }

        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{ChunkingConfig, ChunkingVariant, EvaluationRunOptions};
    use uuid::Uuid;

    fn section_config() -> ChunkingConfig {
        use crate::shared::SectionChunkingConfig;
        ChunkingConfig::Section(SectionChunkingConfig {
            max_section_tokens: 512,
        })
    }

    fn make_request_cmd(run_id: Uuid, dataset_id: Uuid) -> EvaluationRunCommand {
        use super::super::commands::RequestRun;
        EvaluationRunCommand::RequestRun(RequestRun {
            run_id,
            dataset_id,
            pipeline_configuration_id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            document_version: 1,
            variants: vec![ChunkingVariant {
                label: "section-512".to_string(),
                config: section_config(),
            }],
            options: vec![EvaluationRunOptions::default()],
            autotune_request: None,
            scoring_policy: ScoringPolicy::default(),
            occurred_at: "2024-01-01T00:00:00Z".into(),
        })
    }

    fn make_run_requested_event(run_id: Uuid, dataset_id: Uuid) -> EvaluationRunEvent {
        EvaluationRunEvent::RunRequested(RunRequested {
            run_id,
            dataset_id,
            pipeline_configuration_id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            document_version: 1,
            variants: vec![ChunkingVariant {
                label: "section-512".to_string(),
                config: section_config(),
            }],
            options: vec![EvaluationRunOptions::default()],
            autotune_request: None,
            scoring_policy: ScoringPolicy::default(),
            occurred_at: "2024-01-01T00:00:00Z".into(),
        })
    }

    #[test]
    fn request_run_emits_run_requested() {
        let run_id = Uuid::new_v4();
        let dataset_id = Uuid::new_v4();
        let events =
            EvaluationRun::handle_command(None, make_request_cmd(run_id, dataset_id)).unwrap();

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EvaluationRunEvent::RunRequested(_)));

        let run = EvaluationRun::from_events(&events).unwrap();
        assert_eq!(run.run_id, run_id);
        assert_eq!(run.dataset_id, dataset_id);
        assert!(matches!(run.status, EvaluationRunStatus::Pending));
    }

    #[test]
    fn non_run_requested_first_event_returns_none() {
        let run_id = Uuid::new_v4();
        let events = vec![EvaluationRunEvent::RunCompleted(RunCompleted {
            run_id,
            occurred_at: "2024-01-01T00:00:00Z".into(),
        })];
        assert!(EvaluationRun::from_events(&events).is_none());
    }

    #[test]
    fn mark_variant_prepared_is_idempotent() {
        use super::super::commands::MarkVariantPrepared;
        let run_id = Uuid::new_v4();
        let dataset_id = Uuid::new_v4();
        let mut events = vec![make_run_requested_event(run_id, dataset_id)];
        let run = EvaluationRun::from_events(&events).unwrap();

        let cmd = EvaluationRunCommand::MarkVariantPrepared(MarkVariantPrepared {
            variant_label: "section-512".to_string(),
            chunk_set_id: Uuid::new_v4(),
            embedding_set_id: Uuid::new_v4(),
            occurred_at: "2024-01-01T00:01:00Z".into(),
        });
        let new_events = EvaluationRun::handle_command(Some(&run), cmd).unwrap();
        events.extend(new_events);

        let run = EvaluationRun::from_events(&events).unwrap();
        assert_eq!(run.prepared_variants.len(), 1);

        // Second prepare with same label → no-op
        let cmd2 = EvaluationRunCommand::MarkVariantPrepared(MarkVariantPrepared {
            variant_label: "section-512".to_string(),
            chunk_set_id: Uuid::new_v4(),
            embedding_set_id: Uuid::new_v4(),
            occurred_at: "2024-01-01T00:01:30Z".into(),
        });
        let no_op = EvaluationRun::handle_command(Some(&run), cmd2).unwrap();
        assert!(no_op.is_empty());
    }

    #[test]
    fn score_variant_is_idempotent() {
        use super::super::commands::{MarkVariantPrepared, ScoreVariant};
        use crate::shared::{EvaluationMetrics, EvaluationResultSplit};

        let run_id = Uuid::new_v4();
        let dataset_id = Uuid::new_v4();
        let mut events = vec![make_run_requested_event(run_id, dataset_id)];
        let run = EvaluationRun::from_events(&events).unwrap();
        events.extend(
            EvaluationRun::handle_command(
                Some(&run),
                EvaluationRunCommand::MarkVariantPrepared(MarkVariantPrepared {
                    variant_label: "section-512".to_string(),
                    chunk_set_id: Uuid::new_v4(),
                    embedding_set_id: Uuid::new_v4(),
                    occurred_at: "2024-01-01T00:01:00Z".into(),
                }),
            )
            .unwrap(),
        );

        let run = EvaluationRun::from_events(&events).unwrap();
        let metrics = EvaluationMetrics {
            recall_mean: 0.8,
            recall_std: 0.1,
            precision_mean: 0.7,
            precision_std: 0.1,
            iou_mean: 0.6,
            iou_std: 0.05,
            precision_omega_mean: 0.75,
            precision_omega_std: 0.1,
        };
        let score_cmd = ScoreVariant {
            variant_label: "section-512".to_string(),
            split: EvaluationResultSplit::Full,
            metrics: metrics.clone(),
            retrieval_traces: vec![],
            selected: true,
            occurred_at: "2024-01-01T00:02:00Z".into(),
        };
        events.extend(
            EvaluationRun::handle_command(
                Some(&run),
                EvaluationRunCommand::ScoreVariant(score_cmd),
            )
            .unwrap(),
        );

        let run = EvaluationRun::from_events(&events).unwrap();
        assert_eq!(run.scored_variants.len(), 1);

        // Second score same variant+split → no-op
        let dup = EvaluationRun::handle_command(
            Some(&run),
            EvaluationRunCommand::ScoreVariant(ScoreVariant {
                variant_label: "section-512".to_string(),
                split: EvaluationResultSplit::Full,
                metrics,
                retrieval_traces: vec![],
                selected: true,
                occurred_at: "2024-01-01T00:02:30Z".into(),
            }),
        )
        .unwrap();
        assert!(dup.is_empty());
    }

    #[test]
    fn complete_before_all_scored_fails() {
        use super::super::commands::CompleteRun;
        let run_id = Uuid::new_v4();
        let dataset_id = Uuid::new_v4();
        let events = vec![make_run_requested_event(run_id, dataset_id)];
        let run = EvaluationRun::from_events(&events).unwrap();

        let err = EvaluationRun::handle_command(
            Some(&run),
            EvaluationRunCommand::CompleteRun(CompleteRun {
                occurred_at: "2024-01-01T00:05:00Z".into(),
            }),
        )
        .unwrap_err();
        assert!(matches!(err, EvaluationRunError::NotAllVariantsScored));
    }

    #[test]
    fn complete_after_all_scored_succeeds_and_is_idempotent() {
        use super::super::commands::{CompleteRun, MarkVariantPrepared, ScoreVariant};
        use crate::shared::{EvaluationMetrics, EvaluationResultSplit};

        let run_id = Uuid::new_v4();
        let dataset_id = Uuid::new_v4();
        let mut events = vec![make_run_requested_event(run_id, dataset_id)];

        let run = EvaluationRun::from_events(&events).unwrap();
        events.extend(
            EvaluationRun::handle_command(
                Some(&run),
                EvaluationRunCommand::MarkVariantPrepared(MarkVariantPrepared {
                    variant_label: "section-512".to_string(),
                    chunk_set_id: Uuid::new_v4(),
                    embedding_set_id: Uuid::new_v4(),
                    occurred_at: "2024-01-01T00:01:00Z".into(),
                }),
            )
            .unwrap(),
        );

        let run = EvaluationRun::from_events(&events).unwrap();
        let metrics = EvaluationMetrics {
            recall_mean: 0.8,
            recall_std: 0.1,
            precision_mean: 0.7,
            precision_std: 0.1,
            iou_mean: 0.6,
            iou_std: 0.05,
            precision_omega_mean: 0.75,
            precision_omega_std: 0.1,
        };
        events.extend(
            EvaluationRun::handle_command(
                Some(&run),
                EvaluationRunCommand::ScoreVariant(ScoreVariant {
                    variant_label: "section-512".to_string(),
                    split: EvaluationResultSplit::Full,
                    metrics,
                    retrieval_traces: vec![],
                    selected: true,
                    occurred_at: "2024-01-01T00:02:00Z".into(),
                }),
            )
            .unwrap(),
        );

        let run = EvaluationRun::from_events(&events).unwrap();
        let complete_events = EvaluationRun::handle_command(
            Some(&run),
            EvaluationRunCommand::CompleteRun(CompleteRun {
                occurred_at: "2024-01-01T00:03:00Z".into(),
            }),
        )
        .unwrap();
        events.extend(complete_events);

        let run = EvaluationRun::from_events(&events).unwrap();
        assert!(matches!(run.status, EvaluationRunStatus::Completed));

        // Idempotent second complete
        let no_op = EvaluationRun::handle_command(
            Some(&run),
            EvaluationRunCommand::CompleteRun(CompleteRun {
                occurred_at: "2024-01-01T00:04:00Z".into(),
            }),
        )
        .unwrap();
        assert!(no_op.is_empty());
    }

    #[test]
    fn fail_run_is_idempotent() {
        use super::super::commands::FailRun;
        let run_id = Uuid::new_v4();
        let dataset_id = Uuid::new_v4();
        let mut events = vec![make_run_requested_event(run_id, dataset_id)];
        let run = EvaluationRun::from_events(&events).unwrap();
        events.extend(
            EvaluationRun::handle_command(
                Some(&run),
                EvaluationRunCommand::FailRun(FailRun {
                    reason: "embedding timeout".to_string(),
                    occurred_at: "2024-01-01T00:02:00Z".into(),
                }),
            )
            .unwrap(),
        );

        let run = EvaluationRun::from_events(&events).unwrap();
        assert!(matches!(run.status, EvaluationRunStatus::Failed { .. }));

        let no_op = EvaluationRun::handle_command(
            Some(&run),
            EvaluationRunCommand::FailRun(FailRun {
                reason: "still failing".to_string(),
                occurred_at: "2024-01-01T00:03:00Z".into(),
            }),
        )
        .unwrap();
        assert!(no_op.is_empty());
    }

    #[test]
    fn compute_id_is_deterministic() {
        let dataset_id = Uuid::new_v4();
        let pipeline_id = Uuid::new_v4();
        let variants = vec![ChunkingVariant {
            label: "section-512".to_string(),
            config: section_config(),
        }];
        let options = vec![EvaluationRunOptions::default()];

        let id1 = EvaluationRun::compute_id(dataset_id, pipeline_id, &variants, &options, None);
        let id2 = EvaluationRun::compute_id(dataset_id, pipeline_id, &variants, &options, None);
        assert_eq!(id1, id2);
    }

    #[test]
    fn compute_id_differs_for_different_params() {
        let dataset_id = Uuid::new_v4();
        let pipeline_id = Uuid::new_v4();
        let variants_a = vec![ChunkingVariant {
            label: "section-512".to_string(),
            config: section_config(),
        }];
        let variants_b = vec![ChunkingVariant {
            label: "section-256".to_string(),
            config: ChunkingConfig::Section(crate::shared::SectionChunkingConfig {
                max_section_tokens: 256,
            }),
        }];
        let options = vec![EvaluationRunOptions::default()];

        let id_a = EvaluationRun::compute_id(dataset_id, pipeline_id, &variants_a, &options, None);
        let id_b = EvaluationRun::compute_id(dataset_id, pipeline_id, &variants_b, &options, None);
        assert_ne!(id_a, id_b);
    }
}
