#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PostSlug(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostSlugValidationError {
    Empty,
    InvalidFormat(String),
}

impl std::fmt::Display for PostSlugValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Post slug cannot be empty"),
            Self::InvalidFormat(slug) => write!(f, "Invalid post slug: {slug}"),
        }
    }
}

impl std::error::Error for PostSlugValidationError {}

impl PostSlug {
    pub fn parse(raw: &str) -> Result<Self, PostSlugValidationError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(PostSlugValidationError::Empty);
        }

        let valid = trimmed
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid || trimmed.starts_with('-') || trimmed.ends_with('-') || trimmed.contains("--") {
            return Err(PostSlugValidationError::InvalidFormat(trimmed.to_string()));
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PostSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_slug() {
        let slug = PostSlug::parse("quest-exactly-once-p1").unwrap();
        assert_eq!(slug.as_str(), "quest-exactly-once-p1");
    }

    #[test]
    fn trims_valid_slug() {
        let slug = PostSlug::parse("  blog-view-counter  ").unwrap();
        assert_eq!(slug.as_str(), "blog-view-counter");
    }

    #[test]
    fn rejects_invalid_slug() {
        for raw in ["", "My-Post", "../secret", "-bad", "bad-", "bad--slug"] {
            assert!(PostSlug::parse(raw).is_err(), "{raw} should be invalid");
        }
    }
}
