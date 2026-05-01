use std::sync::Arc;

use async_trait::async_trait;

use crate::api_worker::{
    application::{AppError, EmailServiceTrait, RequestValidationServiceTrait},
    domain::ContactMessage,
};

#[async_trait(?Send)]
pub trait ContactMessageServiceTrait {
    async fn new_message(
        &self,
        token: String,
        ip_address: String,
        email: String,
        name: String,
        message: String,
    ) -> Result<(), AppError>;
}

pub struct ContactMessageService {
    pub request_validation_service: Arc<dyn RequestValidationServiceTrait + Send + Sync>,
    pub email_service: Arc<dyn EmailServiceTrait + Send + Sync>,
}

impl ContactMessageService {
    pub fn create(
        request_validation_service: Arc<dyn RequestValidationServiceTrait + Send + Sync>,
        email_service: Arc<dyn EmailServiceTrait + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            request_validation_service,
            email_service,
        })
    }
}

#[async_trait(?Send)]
impl ContactMessageServiceTrait for ContactMessageService {
    async fn new_message(
        &self,
        token: String,
        ip_address: String,
        email: String,
        name: String,
        message: String,
    ) -> Result<(), AppError> {
        if let Err(e) = self
            .request_validation_service
            .verify(token, ip_address)
            .await
        {
            return Err(AppError::Unauthorised(format!(
                "Request validation failed: {e}"
            )));
        }

        let contact_message = ContactMessage::create(email, name, message)
            .map_err(|e| AppError::ValidationError(e.to_string()))?;

        self.email_service
            .forward_contact_message(&contact_message)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockRequestValidationService {
        should_verify_fail: Arc<Mutex<bool>>,
    }

    impl MockRequestValidationService {
        fn new() -> Self {
            Self {
                should_verify_fail: Arc::new(Mutex::new(false)),
            }
        }

        fn set_verify_should_fail(&self, should_fail: bool) {
            *self.should_verify_fail.lock().unwrap() = should_fail;
        }
    }

    #[async_trait(?Send)]
    impl RequestValidationServiceTrait for MockRequestValidationService {
        async fn verify(&self, _token: String, _ip: String) -> Result<(), AppError> {
            if *self.should_verify_fail.lock().unwrap() {
                return Err(AppError::Unauthorised(
                    "Mock verification failed".to_string(),
                ));
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockEmailService {
        should_forward_fail: Arc<Mutex<bool>>,
    }

    impl MockEmailService {
        fn new() -> Self {
            Self {
                should_forward_fail: Arc::new(Mutex::new(false)),
            }
        }

        fn set_forward_should_fail(&self, should_fail: bool) {
            *self.should_forward_fail.lock().unwrap() = should_fail;
        }
    }

    #[async_trait(?Send)]
    impl EmailServiceTrait for MockEmailService {
        async fn forward_contact_message(&self, _message: &ContactMessage) -> Result<(), AppError> {
            if *self.should_forward_fail.lock().unwrap() {
                return Err(AppError::InternalError(
                    "Mock email forwarding failed".to_string(),
                ));
            }
            Ok(())
        }
    }

    fn create_service() -> (
        Arc<ContactMessageService>,
        Arc<MockRequestValidationService>,
        Arc<MockEmailService>,
    ) {
        let mock_validation_service = Arc::new(MockRequestValidationService::new());
        let mock_email_service = Arc::new(MockEmailService::new());
        let service = ContactMessageService::create(
            mock_validation_service.clone(),
            mock_email_service.clone(),
        );
        (service, mock_validation_service, mock_email_service)
    }

    #[tokio::test]
    async fn test_create_message_success() {
        let (service, _, _) = create_service();

        let result = service
            .new_message(
                "mock-token".to_string(),
                "192.168.1.1".to_string(),
                "test@example.com".to_string(),
                "John Doe".to_string(),
                "Test message".to_string(),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_message_sending_error() {
        let (service, _mock_validation, mock_email) = create_service();

        mock_email.set_forward_should_fail(true);

        let result = service
            .new_message(
                "mock-token".to_string(),
                "192.168.1.1".to_string(),
                "test@example.com".to_string(),
                "John Doe".to_string(),
                "Test message".to_string(),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::InternalError(_) => {}
            _ => panic!("Expected InternalError"),
        }
    }

    #[tokio::test]
    async fn test_create_message_request_validation_fails() {
        let (service, mock_validation, _mock_email) = create_service();

        mock_validation.set_verify_should_fail(true);

        let result = service
            .new_message(
                "invalid-token".to_string(),
                "192.168.1.1".to_string(),
                "test@example.com".to_string(),
                "John Doe".to_string(),
                "Test message".to_string(),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Unauthorised(msg) => {
                assert!(msg.contains("Request validation failed"));
            }
            _ => panic!("Expected Unauthorised error"),
        }
    }
}
