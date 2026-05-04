use async_trait::async_trait;
use serde_json::Value;
use tracing::error;
use worker::{ByteStream, Fetch, Headers, Method, Request, RequestInit};

#[async_trait(?Send)]
pub trait HttpClientTrait: Send + Sync {
    async fn post(
        &self,
        url: &str,
        body: Value,
        headers: Vec<(&str, &str)>,
    ) -> Result<Value, String>;

    async fn post_stream(
        &self,
        url: &str,
        body: Value,
        headers: Vec<(&str, &str)>,
    ) -> Result<ByteStream, String> {
        let _ = url;
        let _ = body;
        let _ = headers;
        Err("Not implemented".to_string())
    }

    async fn get(&self, url: &str, headers: Vec<(&str, &str)>) -> Result<Value, String> {
        let _ = url;
        let _ = headers;
        Err("Not implemented".to_string())
    }
}

pub struct WorkerHttpClient;

impl WorkerHttpClient {
    pub fn new() -> Self {
        Self
    }

    fn build_post_request(
        &self,
        url: &str,
        body: Value,
        headers: Vec<(&str, &str)>,
    ) -> Result<Request, String> {
        let body_string = serde_json::to_string(&body)
            .map_err(|e| format!("Failed to serialize request body: {}", e))?;

        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_body(Some(body_string.into()));

        let worker_headers = Headers::new();
        for (name, value) in headers {
            worker_headers
                .set(name, value)
                .map_err(|e| format!("Failed to set header {}: {}", name, e))?;
        }
        init.with_headers(worker_headers);

        Request::new_with_init(url, &init).map_err(|e| format!("Failed to create request: {}", e))
    }

    async fn execute_request(&self, request: Request) -> Result<Value, String> {
        let mut response = Fetch::Request(request).send().await.map_err(|e| {
            error!("HTTP request failed: {:?}", e);
            format!("HTTP request failed: {}", e)
        })?;

        let status = response.status_code();
        if !(200..300).contains(&status) {
            let body = response.text().await.unwrap_or_default();
            error!("API call returned {}: {}", status, body);
            return Err(format!("HTTP {} from upstream: {}", status, body));
        }

        let json_response: Value = response.json().await.map_err(|e| {
            error!("Failed to parse JSON response: {:?}", e);
            format!("Failed to parse JSON response: {}", e)
        })?;

        Ok(json_response)
    }

    async fn execute_stream_request(&self, request: Request) -> Result<ByteStream, String> {
        let mut response = Fetch::Request(request).send().await.map_err(|e| {
            error!("HTTP request failed: {:?}", e);
            format!("HTTP request failed: {}", e)
        })?;

        let status = response.status_code();
        if !(200..300).contains(&status) {
            let body = response.text().await.unwrap_or_default();
            error!("API call returned {}: {}", status, body);
            return Err(format!("HTTP {} from upstream: {}", status, body));
        }

        response.stream().map_err(|e| {
            error!("Failed to read streaming response: {:?}", e);
            format!("Failed to read streaming response: {}", e)
        })
    }
}

impl Default for WorkerHttpClient {
    fn default() -> Self {
        WorkerHttpClient::new()
    }
}

#[async_trait(?Send)]
impl HttpClientTrait for WorkerHttpClient {
    async fn post(
        &self,
        url: &str,
        body: Value,
        headers: Vec<(&str, &str)>,
    ) -> Result<Value, String> {
        let request = self.build_post_request(url, body, headers)?;
        self.execute_request(request).await
    }

    async fn post_stream(
        &self,
        url: &str,
        body: Value,
        headers: Vec<(&str, &str)>,
    ) -> Result<ByteStream, String> {
        let request = self.build_post_request(url, body, headers)?;
        self.execute_stream_request(request).await
    }

    async fn get(&self, url: &str, headers: Vec<(&str, &str)>) -> Result<Value, String> {
        let mut init = RequestInit::new();
        init.with_method(Method::Get);

        let worker_headers = Headers::new();
        for (name, value) in headers {
            worker_headers
                .set(name, value)
                .map_err(|e| format!("Failed to set header {}: {}", name, e))?;
        }
        init.with_headers(worker_headers);

        let request = Request::new_with_init(url, &init)
            .map_err(|e| format!("Failed to create request: {}", e))?;

        self.execute_request(request).await
    }
}
