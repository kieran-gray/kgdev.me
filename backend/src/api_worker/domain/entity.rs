use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api_worker::domain::exceptions::ValidationError;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ContactMessage {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub message: String,
    pub received_at: DateTime<Utc>,
}

impl ContactMessage {
    pub fn create(email: String, name: String, message: String) -> Result<Self, ValidationError> {
        Self::validate_email(&email)?;
        Self::validate_name(&name)?;
        Self::validate_message(&message)?;
        let received_at = Utc::now();

        Ok(Self {
            id: Uuid::new_v4(),
            email,
            name,
            message,
            received_at,
        })
    }

    fn validate_email(email: &str) -> Result<(), ValidationError> {
        if email.is_empty() || email.len() > 254 || email.chars().any(|c| c.is_whitespace()) {
            return Err(ValidationError::InvalidEmail(
                "Email must be between 1 and 254 characters".into(),
            ));
        }

        let mut parts = email.split('@');
        let local = parts.next().unwrap_or("");
        let domain = parts.next().unwrap_or("");

        if parts.next().is_some() || local.is_empty() || domain.is_empty() {
            return Err(ValidationError::InvalidEmail(
                "Invalid email format".to_string(),
            ));
        }

        match domain.find('.') {
            Some(i) if i > 0 && i < domain.len() - 1 => Ok(()),
            _ => Err(ValidationError::InvalidEmail(
                "Invalid email format".to_string(),
            )),
        }
    }

    fn validate_name(name: &str) -> Result<(), ValidationError> {
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err(ValidationError::InvalidName("Name cannot be empty".into()));
        }

        if trimmed.len() > 100 {
            return Err(ValidationError::InvalidName(
                "Name must be 100 characters or less".into(),
            ));
        }

        Ok(())
    }

    fn validate_message(message: &str) -> Result<(), ValidationError> {
        let trimmed = message.trim();

        if trimmed.is_empty() {
            return Err(ValidationError::InvalidMessage(
                "Message cannot be empty".into(),
            ));
        }

        if trimmed.len() > 5000 {
            return Err(ValidationError::InvalidMessage(
                "Message must be 5000 characters or less".into(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_contact_message() {
        let result = ContactMessage::create(
            "test@example.com".to_string(),
            "John Doe".to_string(),
            "This is a valid message".to_string(),
        );

        assert!(result.is_ok());
        let contact = result.unwrap();
        assert_eq!(contact.email, "test@example.com");
        assert_eq!(contact.name, "John Doe");
        assert_eq!(contact.message, "This is a valid message");
    }

    #[test]
    fn test_validate_email_empty() {
        let result = ContactMessage::create(
            "".to_string(),
            "John Doe".to_string(),
            "Valid message here".to_string(),
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidEmail(_)
        ));
    }

    #[test]
    fn test_validate_email_too_long() {
        let long_email = format!("{}@example.com", "a".repeat(250));
        let result = ContactMessage::create(
            long_email,
            "John Doe".to_string(),
            "Valid message here".to_string(),
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidEmail(_)
        ));
    }

    #[test]
    fn test_validate_email_invalid_format() {
        let invalid_emails = vec![
            "notanemail",
            "missing@domain",
            "@nodomain.com",
            "no domain@.com",
            "spaces in@email.com",
        ];

        for email in invalid_emails {
            let result = ContactMessage::create(
                email.to_string(),
                "John Doe".to_string(),
                "Valid message here".to_string(),
            );

            assert!(result.is_err(), "Email '{}' should be invalid", email);
            assert!(matches!(
                result.unwrap_err(),
                ValidationError::InvalidEmail(_)
            ));
        }
    }

    #[test]
    fn test_validate_email_valid_formats() {
        let valid_emails = vec![
            "test@example.com",
            "user.name@example.com",
            "user+tag@example.co.uk",
            "test123@test-domain.com",
        ];

        for email in valid_emails {
            let result = ContactMessage::create(
                email.to_string(),
                "John Doe".to_string(),
                "Valid message here".to_string(),
            );

            assert!(result.is_ok(), "Email '{}' should be valid", email);
        }
    }

    #[test]
    fn test_validate_name_empty() {
        let result = ContactMessage::create(
            "test@example.com".to_string(),
            "".to_string(),
            "Valid message here".to_string(),
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidName(_)
        ));
    }

    #[test]
    fn test_validate_name_whitespace_only() {
        let result = ContactMessage::create(
            "test@example.com".to_string(),
            "   ".to_string(),
            "Valid message here".to_string(),
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidName(_)
        ));
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(101);
        let result = ContactMessage::create(
            "test@example.com".to_string(),
            long_name,
            "Valid message here".to_string(),
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidName(_)
        ));
    }

    #[test]
    fn test_validate_message_empty() {
        let result = ContactMessage::create(
            "test@example.com".to_string(),
            "John Doe".to_string(),
            "".to_string(),
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidMessage(_)
        ));
    }

    #[test]
    fn test_validate_message_whitespace_only() {
        let result = ContactMessage::create(
            "test@example.com".to_string(),
            "John Doe".to_string(),
            "     ".to_string(),
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidMessage(_)
        ));
    }

    #[test]
    fn test_validate_message_too_long() {
        let long_message = "a".repeat(5001);
        let result = ContactMessage::create(
            "test@example.com".to_string(),
            "John Doe".to_string(),
            long_message,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidMessage(_)
        ));
    }
}
