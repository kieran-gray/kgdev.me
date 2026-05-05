use crate::server::application::evaluation::ports::{
    EvaluationPrompt, GeneratedEvaluationQuestion,
};
use crate::server::application::AppError;
use crate::shared::{EvaluationQuestion, EvaluationReference};

const SOURCE_WINDOW_CHARS: usize = 4000;
const MAX_REFERENCE_COUNT: usize = 5;

pub fn build_question_prompt(
    source_window: &str,
    previous_questions: &[String],
) -> EvaluationPrompt {
    let previous = if previous_questions.is_empty() {
        "None".to_string()
    } else {
        previous_questions.join("\n")
    };

    EvaluationPrompt {
        system: concat!(
            "You generate retrieval evaluation questions from supplied blog text. ",
            "Generate one question that can be answered only by facts in the text. ",
            "Return JSON with exactly these keys: question, references. ",
            "references must be an array of exact copied excerpts from the supplied text. ",
            "Prefer whole sentences. Use no more than five references."
        )
        .to_string(),
        user: format!(
            "Text:\n{source_window}\n\nPrevious questions to avoid:\n{previous}\n\nReturn only JSON."
        ),
    }
}

pub fn generated_to_question(
    generated: &GeneratedEvaluationQuestion,
    document: &str,
) -> Result<EvaluationQuestion, AppError> {
    if generated.references.len() > MAX_REFERENCE_COUNT {
        return Err(AppError::Validation(format!(
            "too many references: {}",
            generated.references.len()
        )));
    }

    let mut references = Vec::with_capacity(generated.references.len());
    for reference in &generated.references {
        let (start, end) = find_reference(document, reference).ok_or_else(|| {
            AppError::Validation(format!(
                "reference was not found in document: {}",
                truncate(reference, 120)
            ))
        })?;
        references.push(EvaluationReference {
            content: document[start..end].to_string(),
            char_start: byte_to_char_index(document, start) as u32,
            char_end: byte_to_char_index(document, end) as u32,
        });
    }

    Ok(EvaluationQuestion {
        question: generated.question.clone(),
        references,
    })
}

pub fn sample_window(document: &str, attempt: usize, max_attempts: usize) -> String {
    let chars: Vec<char> = document.chars().collect();
    if chars.len() <= SOURCE_WINDOW_CHARS {
        return document.to_string();
    }

    let max_start = chars.len().saturating_sub(SOURCE_WINDOW_CHARS);
    let denominator = max_attempts.saturating_sub(1).max(1);
    let start = max_start * attempt / denominator;
    chars[start..start + SOURCE_WINDOW_CHARS].iter().collect()
}

fn find_reference(document: &str, reference: &str) -> Option<(usize, usize)> {
    if let Some(start) = document.find(reference) {
        return Some((start, start + reference.len()));
    }
    find_despite_whitespace(document, reference)
}

fn find_despite_whitespace(document: &str, reference: &str) -> Option<(usize, usize)> {
    let words: Vec<&str> = reference.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }

    for (start, _) in document.char_indices() {
        let mut pos = start;
        let mut matched = true;
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                pos = skip_whitespace(document, pos);
            }
            if pos > document.len() || !document[pos..].starts_with(word) {
                matched = false;
                break;
            }
            pos += word.len();
        }
        if matched {
            return Some((start, pos));
        }
    }
    None
}

fn skip_whitespace(document: &str, mut pos: usize) -> usize {
    while pos < document.len() {
        let Some(ch) = document[pos..].chars().next() else {
            break;
        };
        if !ch.is_whitespace() {
            break;
        }
        pos += ch.len_utf8();
    }
    pos
}

fn byte_to_char_index(document: &str, byte_index: usize) -> usize {
    document[..byte_index].chars().count()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_exact_reference() {
        let doc = "alpha beta gamma";
        assert_eq!(find_reference(doc, "beta"), Some((6, 10)));
    }

    #[test]
    fn finds_reference_despite_whitespace() {
        let doc = "alpha beta\n\ngamma";
        assert_eq!(find_reference(doc, "beta gamma"), Some((6, 17)));
    }

    #[test]
    fn generated_question_uses_char_offsets() {
        let doc = "£ alpha beta";
        let generated = GeneratedEvaluationQuestion {
            question: "What word follows alpha?".into(),
            references: vec!["alpha beta".into()],
        };

        let question = generated_to_question(&generated, doc).unwrap();

        assert_eq!(question.references[0].char_start, 2);
        assert_eq!(question.references[0].char_end, 12);
    }
}
