use crate::api_worker::domain::{
    QuestionValidationError,
    question::constants::{MAX_QUESTION_CHARS, MIN_QUESTION_CHARS},
};

use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

pub struct Question {
    pub question: String,
}

impl Question {
    pub fn create(question: &str) -> Result<Self, QuestionValidationError> {
        let trimmed = question.trim();
        if trimmed.len() < MIN_QUESTION_CHARS {
            return Err(QuestionValidationError::TooShort);
        }
        if trimmed.len() > MAX_QUESTION_CHARS {
            return Err(QuestionValidationError::TooLong);
        }

        let normalised = Question::normalise(trimmed);
        if normalised.len() < MIN_QUESTION_CHARS {
            return Err(QuestionValidationError::TooShort);
        }
        Ok(Self {
            question: normalised,
        })
    }

    fn normalise(question: &str) -> String {
        let nfkc: String = question.nfkc().collect();
        let lowered = nfkc.to_lowercase();
        let collapsed: String = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
        collapsed
            .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
            .to_string()
    }

    pub fn as_str(&self) -> &str {
        &self.question
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.question.as_bytes());
        let bytes = hasher.finalize();
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_lowercases_trims_collapses() {
        let q = Question::create("  How DOES Hibernation\twork? ").unwrap();
        assert_eq!(q.as_str(), "how does hibernation work");
    }

    #[test]
    fn normalise_strips_outer_punctuation() {
        let q = Question::create("???what now???").unwrap();
        assert_eq!(q.as_str(), "what now");
    }

    #[test]
    fn hash_is_stable() {
        let q1 = Question::create("how does hibernation work").unwrap();
        let q2 = Question::create("how does hibernation work").unwrap();

        assert_eq!(q1.hash(), q2.hash());
        assert_eq!(q1.hash().len(), 64);
    }

    #[test]
    fn hash_is_sensitive_to_content() {
        let q1 = Question::create("hibernation").unwrap();
        let q2 = Question::create("hibernate").unwrap();
        assert_ne!(q1.hash(), q2.hash());
    }

    #[test]
    fn normalise_applies_nfkc_full_width() {
        let q = Question::create("ＡＢＣ").unwrap();
        assert_eq!(q.as_str(), "abc");
    }

    #[test]
    fn normalise_applies_nfkc_ligature() {
        let q1 = Question::create("ef\u{FB01}cient").unwrap();
        let q2 = Question::create("efficient").unwrap();
        assert_eq!(q1.as_str(), q2.as_str());
    }

    #[test]
    fn validate_rejects_short() {
        assert!(matches!(
            Question::create("i"),
            Err(QuestionValidationError::TooShort)
        ));

        assert!(matches!(
            Question::create("??? ???"),
            Err(QuestionValidationError::TooShort)
        ));
    }

    #[test]
    fn validate_rejects_long() {
        let s = "x".repeat(MAX_QUESTION_CHARS + 1);
        assert!(matches!(
            Question::create(&s),
            Err(QuestionValidationError::TooLong)
        ));
    }
}
