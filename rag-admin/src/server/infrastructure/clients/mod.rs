pub mod cloudflare;
pub mod ollama;

pub use cloudflare::{CloudflareApi, CloudflareCredentials, CLOUDFLARE_API_BASE};
pub use ollama::{OllamaApi, OLLAMA_API_BASE};
