use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::shared::Timestamp;
use crate::server::domain::Aggregate;

use super::super::question::EvaluationQuestion;
use super::{
    commands::EvaluationDatasetCommand,
    events::{
        DatasetGenerationCompleted, DatasetGenerationFailed, DatasetGenerationRequested,
        EvaluationDatasetEvent, QuestionAccepted, QuestionRejected,
    },
    exceptions::EvaluationDatasetError,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatasetGenerationStatus {
    Generating,
    Completed,
    Failed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationDataset {
    pub dataset_id: Uuid,
    pub document_id: Uuid,
    pub document_version: u32,
    pub content_hash: String,
    pub label: String,
    pub target_question_count: u32,
    pub generation_model: String,
    pub generation_backend: String,
    pub excerpt_similarity_threshold_milli: u32,
    pub duplicate_similarity_threshold_milli: u32,
    pub embedding_model_id: Uuid,
    pub questions: Vec<EvaluationQuestion>,
    pub rejection_count: u32,
    pub status: DatasetGenerationStatus,
    pub created_at: Timestamp,
}

impl EvaluationDataset {
    fn from_requested(e: &DatasetGenerationRequested) -> Self {
        Self {
            dataset_id: e.dataset_id,
            document_id: e.document_id,
            document_version: e.document_version,
            content_hash: e.content_hash.clone(),
            label: e.label.clone(),
            target_question_count: e.target_question_count,
            generation_model: e.generation_model.clone(),
            generation_backend: e.generation_backend.clone(),
            excerpt_similarity_threshold_milli: e.excerpt_similarity_threshold_milli,
            duplicate_similarity_threshold_milli: e.duplicate_similarity_threshold_milli,
            embedding_model_id: e.embedding_model_id,
            questions: Vec::new(),
            rejection_count: 0,
            status: DatasetGenerationStatus::Generating,
            created_at: e.occurred_at.clone(),
        }
    }
}

impl Aggregate for EvaluationDataset {
    type Event = EvaluationDatasetEvent;
    type Command = EvaluationDatasetCommand;
    type Error = EvaluationDatasetError;

    fn aggregate_id(&self) -> String {
        self.dataset_id.to_string()
    }

    fn apply(&mut self, event: &Self::Event) {
        match event {
            Self::Event::DatasetGenerationRequested(_) => {}
            Self::Event::QuestionAccepted(e) => {
                self.questions.push(EvaluationQuestion {
                    sequence: e.sequence,
                    question: e.question.clone(),
                    references: e.references.clone(),
                    embedding: e.embedding.clone(),
                });
            }
            Self::Event::QuestionRejected(_) => {
                self.rejection_count += 1;
            }
            Self::Event::DatasetGenerationCompleted(_) => {
                self.status = DatasetGenerationStatus::Completed;
            }
            Self::Event::DatasetGenerationFailed(e) => {
                self.status = DatasetGenerationStatus::Failed {
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
            Self::Command::RequestDatasetGeneration(cmd) => {
                if state.is_some() {
                    return Err(EvaluationDatasetError::AlreadyExists);
                }
                Ok(vec![Self::Event::DatasetGenerationRequested(
                    DatasetGenerationRequested {
                        dataset_id: cmd.dataset_id,
                        document_id: cmd.document_id,
                        document_version: cmd.document_version,
                        content_hash: cmd.content_hash,
                        label: cmd.label,
                        target_question_count: cmd.target_question_count,
                        generation_model: cmd.generation_model,
                        generation_backend: cmd.generation_backend,
                        excerpt_similarity_threshold_milli: cmd.excerpt_similarity_threshold_milli,
                        duplicate_similarity_threshold_milli: cmd
                            .duplicate_similarity_threshold_milli,
                        embedding_model_id: cmd.embedding_model_id,
                        occurred_at: cmd.occurred_at,
                    },
                )])
            }

            Self::Command::AcceptQuestion(cmd) => {
                let dataset = state.ok_or(EvaluationDatasetError::NotFound)?;
                if !matches!(dataset.status, DatasetGenerationStatus::Generating) {
                    return Err(EvaluationDatasetError::GenerationNotInProgress);
                }
                Ok(vec![Self::Event::QuestionAccepted(QuestionAccepted {
                    dataset_id: dataset.dataset_id,
                    sequence: cmd.sequence,
                    question: cmd.question,
                    references: cmd.references,
                    embedding: cmd.embedding,
                    occurred_at: cmd.occurred_at,
                })])
            }

            Self::Command::RejectQuestion(cmd) => {
                let dataset = state.ok_or(EvaluationDatasetError::NotFound)?;
                if !matches!(dataset.status, DatasetGenerationStatus::Generating) {
                    return Err(EvaluationDatasetError::GenerationNotInProgress);
                }
                Ok(vec![Self::Event::QuestionRejected(QuestionRejected {
                    dataset_id: dataset.dataset_id,
                    attempt: cmd.attempt,
                    reason: cmd.reason,
                    occurred_at: cmd.occurred_at,
                })])
            }

            Self::Command::CompleteDatasetGeneration(cmd) => {
                let dataset = state.ok_or(EvaluationDatasetError::NotFound)?;
                match &dataset.status {
                    DatasetGenerationStatus::Completed => return Ok(vec![]),
                    DatasetGenerationStatus::Failed { .. } => {
                        return Err(EvaluationDatasetError::AlreadyFailed)
                    }
                    DatasetGenerationStatus::Generating => {}
                }
                if dataset.questions.is_empty() {
                    return Err(EvaluationDatasetError::NoQuestionsAccepted);
                }
                Ok(vec![Self::Event::DatasetGenerationCompleted(
                    DatasetGenerationCompleted {
                        dataset_id: dataset.dataset_id,
                        occurred_at: cmd.occurred_at,
                    },
                )])
            }

            Self::Command::FailDatasetGeneration(cmd) => {
                let dataset = state.ok_or(EvaluationDatasetError::NotFound)?;
                match &dataset.status {
                    DatasetGenerationStatus::Completed => {
                        return Err(EvaluationDatasetError::AlreadyCompleted)
                    }
                    DatasetGenerationStatus::Failed { .. } => return Ok(vec![]),
                    DatasetGenerationStatus::Generating => {}
                }
                Ok(vec![Self::Event::DatasetGenerationFailed(
                    DatasetGenerationFailed {
                        dataset_id: dataset.dataset_id,
                        reason: cmd.reason,
                        occurred_at: cmd.occurred_at,
                    },
                )])
            }
        }
    }

    fn from_events(events: &[Self::Event]) -> Option<Self> {
        let mut state: Option<Self> = None;

        for event in events {
            match (&mut state, event) {
                (None, Self::Event::DatasetGenerationRequested(e)) => {
                    state = Some(Self::from_requested(e));
                }
                (Some(_), Self::Event::DatasetGenerationRequested(_)) => return None,
                (None, _) => return None,
                (Some(dataset), event) => dataset.apply(event),
            }
        }

        state
    }
}

#[cfg(test)]
mod tests {
    use super::super::commands::{
        AcceptQuestion, CompleteDatasetGeneration, FailDatasetGeneration, RejectQuestion,
        RequestDatasetGeneration,
    };
    use super::*;
    use uuid::Uuid;

    fn make_request_cmd(dataset_id: Uuid, document_id: Uuid) -> EvaluationDatasetCommand {
        EvaluationDatasetCommand::RequestDatasetGeneration(RequestDatasetGeneration {
            dataset_id,
            document_id,
            document_version: 1,
            content_hash: "abc123".to_string(),
            label: "synthetic-default".to_string(),
            target_question_count: 8,
            generation_model: "llama3".to_string(),
            generation_backend: "ollama".to_string(),
            excerpt_similarity_threshold_milli: 800,
            duplicate_similarity_threshold_milli: 950,
            embedding_model_id: Uuid::new_v4(),
            occurred_at: "2024-01-01T00:00:00Z".into(),
        })
    }

    fn make_requested_event(dataset_id: Uuid, document_id: Uuid) -> EvaluationDatasetEvent {
        EvaluationDatasetEvent::DatasetGenerationRequested(DatasetGenerationRequested {
            dataset_id,
            document_id,
            document_version: 1,
            content_hash: "abc123".to_string(),
            label: "synthetic-default".to_string(),
            target_question_count: 8,
            generation_model: "llama3".to_string(),
            generation_backend: "ollama".to_string(),
            excerpt_similarity_threshold_milli: 800,
            duplicate_similarity_threshold_milli: 950,
            embedding_model_id: Uuid::new_v4(),
            occurred_at: "2024-01-01T00:00:00Z".into(),
        })
    }

    use crate::server::domain::evaluation::question::EvaluationReference;

    fn make_accept_cmd(sequence: u32) -> EvaluationDatasetCommand {
        EvaluationDatasetCommand::AcceptQuestion(AcceptQuestion {
            sequence,
            question: format!("Question {sequence}?"),
            references: vec![EvaluationReference {
                content: "Some content".to_string(),
                char_start: 0,
                char_end: 12,
                embedding: None,
            }],
            embedding: None,
            occurred_at: "2024-01-01T00:01:00Z".into(),
        })
    }

    #[test]
    fn request_generation_emits_requested_event() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let events =
            EvaluationDataset::handle_command(None, make_request_cmd(dataset_id, document_id))
                .unwrap();

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            EvaluationDatasetEvent::DatasetGenerationRequested(_)
        ));

        let dataset = EvaluationDataset::from_events(&events).unwrap();
        assert_eq!(dataset.dataset_id, dataset_id);
        assert_eq!(dataset.document_id, document_id);
        assert_eq!(dataset.questions.len(), 0);
        assert!(matches!(
            dataset.status,
            DatasetGenerationStatus::Generating
        ));
    }

    #[test]
    fn requesting_existing_dataset_fails_with_already_exists() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let events = vec![make_requested_event(dataset_id, document_id)];
        let dataset = EvaluationDataset::from_events(&events).unwrap();

        let err = EvaluationDataset::handle_command(
            Some(&dataset),
            make_request_cmd(dataset_id, document_id),
        )
        .unwrap_err();

        assert!(matches!(err, EvaluationDatasetError::AlreadyExists));
    }

    #[test]
    fn non_requested_first_event_returns_none() {
        let dataset_id = Uuid::new_v4();
        let events = vec![EvaluationDatasetEvent::DatasetGenerationCompleted(
            DatasetGenerationCompleted {
                dataset_id,
                occurred_at: "2024-01-01T00:00:00Z".into(),
            },
        )];
        assert!(EvaluationDataset::from_events(&events).is_none());
    }

    #[test]
    fn accept_question_adds_to_questions_list() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let mut events = vec![make_requested_event(dataset_id, document_id)];
        let dataset = EvaluationDataset::from_events(&events).unwrap();

        let new_events =
            EvaluationDataset::handle_command(Some(&dataset), make_accept_cmd(0)).unwrap();
        assert_eq!(new_events.len(), 1);

        events.extend(new_events);
        let dataset = EvaluationDataset::from_events(&events).unwrap();
        assert_eq!(dataset.questions.len(), 1);
        assert_eq!(dataset.questions[0].sequence, 0);
    }

    #[test]
    fn reject_question_increments_rejection_count() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let mut events = vec![make_requested_event(dataset_id, document_id)];
        let dataset = EvaluationDataset::from_events(&events).unwrap();

        let reject_cmd = EvaluationDatasetCommand::RejectQuestion(RejectQuestion {
            attempt: 1,
            reason: "too similar to existing question".to_string(),
            occurred_at: "2024-01-01T00:01:00Z".into(),
        });
        let new_events = EvaluationDataset::handle_command(Some(&dataset), reject_cmd).unwrap();
        events.extend(new_events);

        let dataset = EvaluationDataset::from_events(&events).unwrap();
        assert_eq!(dataset.rejection_count, 1);
        assert_eq!(dataset.questions.len(), 0);
    }

    #[test]
    fn complete_without_questions_fails() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let events = vec![make_requested_event(dataset_id, document_id)];
        let dataset = EvaluationDataset::from_events(&events).unwrap();

        let err = EvaluationDataset::handle_command(
            Some(&dataset),
            EvaluationDatasetCommand::CompleteDatasetGeneration(CompleteDatasetGeneration {
                occurred_at: "2024-01-01T00:02:00Z".into(),
            }),
        )
        .unwrap_err();

        assert!(matches!(err, EvaluationDatasetError::NoQuestionsAccepted));
    }

    #[test]
    fn complete_with_questions_transitions_to_completed() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let mut events = vec![make_requested_event(dataset_id, document_id)];
        let dataset = EvaluationDataset::from_events(&events).unwrap();
        events
            .extend(EvaluationDataset::handle_command(Some(&dataset), make_accept_cmd(0)).unwrap());

        let dataset = EvaluationDataset::from_events(&events).unwrap();
        let complete_events = EvaluationDataset::handle_command(
            Some(&dataset),
            EvaluationDatasetCommand::CompleteDatasetGeneration(CompleteDatasetGeneration {
                occurred_at: "2024-01-01T00:02:00Z".into(),
            }),
        )
        .unwrap();
        events.extend(complete_events);

        let dataset = EvaluationDataset::from_events(&events).unwrap();
        assert!(matches!(dataset.status, DatasetGenerationStatus::Completed));
    }

    #[test]
    fn complete_already_completed_is_idempotent() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let mut events = vec![make_requested_event(dataset_id, document_id)];
        let dataset = EvaluationDataset::from_events(&events).unwrap();
        events
            .extend(EvaluationDataset::handle_command(Some(&dataset), make_accept_cmd(0)).unwrap());
        let dataset = EvaluationDataset::from_events(&events).unwrap();
        events.extend(
            EvaluationDataset::handle_command(
                Some(&dataset),
                EvaluationDatasetCommand::CompleteDatasetGeneration(CompleteDatasetGeneration {
                    occurred_at: "2024-01-01T00:02:00Z".into(),
                }),
            )
            .unwrap(),
        );

        let dataset = EvaluationDataset::from_events(&events).unwrap();
        let second_complete = EvaluationDataset::handle_command(
            Some(&dataset),
            EvaluationDatasetCommand::CompleteDatasetGeneration(CompleteDatasetGeneration {
                occurred_at: "2024-01-01T00:03:00Z".into(),
            }),
        )
        .unwrap();
        assert!(
            second_complete.is_empty(),
            "re-completing should be a no-op"
        );
    }

    #[test]
    fn fail_transitions_to_failed() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let events = vec![make_requested_event(dataset_id, document_id)];
        let dataset = EvaluationDataset::from_events(&events).unwrap();

        let fail_events = EvaluationDataset::handle_command(
            Some(&dataset),
            EvaluationDatasetCommand::FailDatasetGeneration(FailDatasetGeneration {
                reason: "LLM unavailable".to_string(),
                occurred_at: "2024-01-01T00:02:00Z".into(),
            }),
        )
        .unwrap();

        let all_events: Vec<_> = events.into_iter().chain(fail_events).collect();
        let dataset = EvaluationDataset::from_events(&all_events).unwrap();
        assert!(matches!(
            dataset.status,
            DatasetGenerationStatus::Failed { .. }
        ));
    }

    #[test]
    fn fail_already_failed_is_idempotent() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let mut events = vec![make_requested_event(dataset_id, document_id)];
        let dataset = EvaluationDataset::from_events(&events).unwrap();
        events.extend(
            EvaluationDataset::handle_command(
                Some(&dataset),
                EvaluationDatasetCommand::FailDatasetGeneration(FailDatasetGeneration {
                    reason: "LLM unavailable".to_string(),
                    occurred_at: "2024-01-01T00:02:00Z".into(),
                }),
            )
            .unwrap(),
        );

        let dataset = EvaluationDataset::from_events(&events).unwrap();
        let second_fail = EvaluationDataset::handle_command(
            Some(&dataset),
            EvaluationDatasetCommand::FailDatasetGeneration(FailDatasetGeneration {
                reason: "still unavailable".to_string(),
                occurred_at: "2024-01-01T00:03:00Z".into(),
            }),
        )
        .unwrap();
        assert!(second_fail.is_empty(), "re-failing should be a no-op");
    }

    #[test]
    fn accept_question_after_completion_fails() {
        let dataset_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let mut events = vec![make_requested_event(dataset_id, document_id)];
        let dataset = EvaluationDataset::from_events(&events).unwrap();
        events
            .extend(EvaluationDataset::handle_command(Some(&dataset), make_accept_cmd(0)).unwrap());
        let dataset = EvaluationDataset::from_events(&events).unwrap();
        events.extend(
            EvaluationDataset::handle_command(
                Some(&dataset),
                EvaluationDatasetCommand::CompleteDatasetGeneration(CompleteDatasetGeneration {
                    occurred_at: "2024-01-01T00:02:00Z".into(),
                }),
            )
            .unwrap(),
        );

        let dataset = EvaluationDataset::from_events(&events).unwrap();
        let err =
            EvaluationDataset::handle_command(Some(&dataset), make_accept_cmd(1)).unwrap_err();
        assert!(matches!(
            err,
            EvaluationDatasetError::GenerationNotInProgress
        ));
    }
}
