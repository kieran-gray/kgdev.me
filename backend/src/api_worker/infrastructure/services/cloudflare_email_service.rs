use std::sync::Arc;

use async_trait::async_trait;

use crate::api_worker::{
    application::{AppError, EmailServiceTrait},
    domain::entity::ContactMessage,
    infrastructure::http_client::HttpClientTrait,
};

pub struct CloudflareEmailService {
    cloudflare_account_id: String,
    cloudflare_api_token: String,
    destination_email: String,
    http_client: Arc<dyn HttpClientTrait>,
}

impl CloudflareEmailService {
    pub fn create(
        cloudflare_account_id: String,
        cloudflare_api_token: String,
        destination_email: String,
        http_client: Arc<dyn HttpClientTrait>,
    ) -> Arc<Self> {
        Arc::new(Self {
            cloudflare_account_id,
            cloudflare_api_token,
            destination_email,
            http_client,
        })
    }

    fn escape_html(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }

    fn build_html(message: &ContactMessage) -> String {
        let name = Self::escape_html(&message.name);
        let email = Self::escape_html(&message.email);
        let body = Self::escape_html(&message.message);
        let received_at = message
            .received_at
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <style>
    body {{ font-family: monospace; max-width: 600px; margin: 0 auto; padding: 20px; color: #333; }}
    h2 {{ border-bottom: 2px solid #333; padding-bottom: 8px; margin-bottom: 16px; }}
    table {{ margin-bottom: 20px; border-collapse: collapse; }}
    td {{ padding: 4px 8px 4px 0; vertical-align: top; }}
    td:first-child {{ font-weight: bold; color: #555; white-space: nowrap; padding-right: 16px; }}
    .message {{ border-left: 3px solid #999; padding: 8px 12px; white-space: pre-wrap; word-break: break-word; }}
    .footer {{ color: #999; font-size: 0.85em; border-top: 1px solid #eee; margin-top: 24px; padding-top: 8px; }}
  </style>
</head>
<body>
  <h2>New Contact Message — kgdev.me</h2>
  <table>
    <tr><td>From:</td><td>{name} &lt;{email}&gt;</td></tr>
    <tr><td>Received:</td><td>{received_at}</td></tr>
  </table>
  <div class="message">{body}</div>
  <div class="footer">Sent via the contact form at kgdev.me</div>
</body>
</html>"#
        )
    }

    fn build_text(message: &ContactMessage) -> String {
        let received_at = message
            .received_at
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        format!(
            "New Contact Message — kgdev.me\n\
             ================================\n\n\
             From:     {} <{}>\n\
             Received: {}\n\n\
             Message:\n\
             --------\n\
             {}\n\n\
             ---\n\
             Sent via the contact form at kgdev.me",
            message.name, message.email, received_at, message.message
        )
    }
}

#[async_trait(?Send)]
impl EmailServiceTrait for CloudflareEmailService {
    async fn forward_contact_message(&self, message: &ContactMessage) -> Result<(), AppError> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/email/sending/send",
            self.cloudflare_account_id
        );
        let token = format!("Bearer {}", self.cloudflare_api_token);

        let headers = vec![
            ("Authorization", token.as_str()),
            ("Content-Type", "application/json"),
        ];
        let body = serde_json::json!({
            "to": &self.destination_email,
            "from": "contact@kgdev.me",
            "subject": format!("New message from {} — kgdev.me", message.name),
            "html": Self::build_html(message),
            "text": Self::build_text(message),
        });

        self.http_client
            .post(&url, body, headers)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to send email via Cloudflare: {}", e))
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Mutex;

    struct MockHttpClient {
        should_fail: Arc<Mutex<bool>>,
        last_body: Arc<Mutex<Option<Value>>>,
    }

    impl MockHttpClient {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                should_fail: Arc::new(Mutex::new(false)),
                last_body: Arc::new(Mutex::new(None)),
            })
        }

        fn set_should_fail(&self, fail: bool) {
            *self.should_fail.lock().unwrap() = fail;
        }

        fn last_body(&self) -> Option<Value> {
            self.last_body.lock().unwrap().clone()
        }
    }

    #[async_trait(?Send)]
    impl HttpClientTrait for MockHttpClient {
        async fn post(
            &self,
            _url: &str,
            body: Value,
            _headers: Vec<(&str, &str)>,
        ) -> Result<Value, String> {
            *self.last_body.lock().unwrap() = Some(body);
            if *self.should_fail.lock().unwrap() {
                return Err("Mock HTTP error".to_string());
            }
            Ok(serde_json::json!({ "success": true }))
        }
    }

    fn make_service(client: Arc<MockHttpClient>) -> Arc<CloudflareEmailService> {
        CloudflareEmailService::create(
            "account123".to_string(),
            "token456".to_string(),
            "me@example.com".to_string(),
            client,
        )
    }

    fn make_message() -> ContactMessage {
        ContactMessage::create(
            "sender@example.com".to_string(),
            "Jane Doe".to_string(),
            "Hello, this is a test message.".to_string(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_forward_contact_message_success() {
        let client = MockHttpClient::new();
        let service = make_service(client.clone());
        let message = make_message();

        let result = service.forward_contact_message(&message).await;

        assert!(result.is_ok());
        let body = client.last_body().unwrap();
        assert_eq!(body["to"], "me@example.com");
        assert_eq!(body["from"], "contact@kgdev.me");
        assert!(body["subject"].as_str().unwrap().contains("Jane Doe"));
        let html = body["html"].as_str().unwrap();
        assert!(html.contains("Jane Doe"));
        assert!(html.contains("sender@example.com"));
        assert!(html.contains("Hello, this is a test message."));
        let text = body["text"].as_str().unwrap();
        assert!(text.contains("Jane Doe"));
        assert!(text.contains("sender@example.com"));
        assert!(text.contains("Hello, this is a test message."));
    }

    #[tokio::test]
    async fn test_forward_contact_message_http_error() {
        let client = MockHttpClient::new();
        client.set_should_fail(true);
        let service = make_service(client);
        let message = make_message();

        let result = service.forward_contact_message(&message).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::InternalError(msg) => assert!(msg.contains("Failed to send email")),
            _ => panic!("Expected InternalError"),
        }
    }

    #[tokio::test]
    async fn test_html_escaping() {
        let client = MockHttpClient::new();
        let service = make_service(client.clone());
        let message = ContactMessage::create(
            "test@example.com".to_string(),
            "John <Doe> & Co".to_string(),
            "Hello & goodbye <script>alert('xss')</script>".to_string(),
        )
        .unwrap();

        let result = service.forward_contact_message(&message).await;

        assert!(result.is_ok());
        let body = client.last_body().unwrap();
        let html = body["html"].as_str().unwrap();
        assert!(html.contains("John &lt;Doe&gt; &amp; Co"));
        assert!(html.contains("Hello &amp; goodbye &lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[tokio::test]
    async fn test_correct_cloudflare_url() {
        let client = MockHttpClient::new();

        struct CapturingClient {
            last_url: Arc<Mutex<Option<String>>>,
        }

        #[async_trait(?Send)]
        impl HttpClientTrait for CapturingClient {
            async fn post(
                &self,
                url: &str,
                _body: Value,
                _headers: Vec<(&str, &str)>,
            ) -> Result<Value, String> {
                *self.last_url.lock().unwrap() = Some(url.to_string());
                Ok(serde_json::json!({}))
            }
        }

        let capturing_client = Arc::new(CapturingClient {
            last_url: Arc::new(Mutex::new(None)),
        });
        let service = CloudflareEmailService::create(
            "myaccount".to_string(),
            "mytoken".to_string(),
            "dest@example.com".to_string(),
            capturing_client.clone(),
        );

        let _ = service.forward_contact_message(&make_message()).await;
        let _ = client; // suppress unused warning

        let url = capturing_client.last_url.lock().unwrap().clone().unwrap();
        assert!(url.contains("myaccount"));
        assert!(url.contains("/email/sending/send"));
    }
}
