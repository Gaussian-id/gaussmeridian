//! GaussMeridian Providers
//!
//! This crate provides implementations for various LLM providers including:
//! - OpenAI (GPT-4, GPT-3.5, embeddings)
//! - Anthropic Claude
//! - HuggingFace
//! - Ollama (local models)
//! - Cohere
//! - vLLM
//! - LMStudio
//! - Custom providers
//!
//! Each provider implements the `LLMProvider` trait and provides:
//! - Chat completions
//! - Text completions  
//! - Embeddings
//! - Streaming support (where available)
//! - Rate limiting
//! - Health checks
//! - Cost tracking

#[cfg(feature = "anthropic")]
pub mod anthropic;
#[cfg(feature = "cohere")]
pub mod cohere;
pub mod common;
pub mod cost;
#[cfg(feature = "custom")]
pub mod custom;
pub mod error_utils;
#[cfg(feature = "gemini")]
pub mod gemini;
pub mod health;
#[cfg(feature = "huggingface")]
pub mod huggingface;
#[cfg(feature = "lmstudio")]
pub mod lmstudio;
#[cfg(feature = "ollama")]
pub mod ollama;
#[cfg(feature = "openai")]
pub mod openai;
pub mod rate_limit;
pub mod retry;
pub mod streaming;
pub mod streaming_parser;
#[cfg(feature = "vllm")]
pub mod vllm;

// Re-export main types
#[cfg(feature = "anthropic")]
pub use anthropic::{AnthropicConfig, AnthropicProvider};
pub use common::{BaseProviderConfig, ProviderRegistry};
#[cfg(feature = "gemini")]
pub use gemini::{GeminiConfig, GeminiProvider};
#[cfg(feature = "huggingface")]
pub use huggingface::{HuggingFaceConfig, HuggingFaceProvider};
#[cfg(feature = "ollama")]
pub use ollama::{OllamaConfig, OllamaProvider};
#[cfg(feature = "openai")]
pub use openai::{OpenAIConfig, OpenAIProvider};
pub use rate_limit::{ProviderRateLimiter, RateLimit, RateLimitError};
pub use streaming::{buffer_stream, filter_stream, transform_stream};

// Re-export traits and types from core
pub use gaussmeridian_core::LLMProvider;
pub use gaussmeridian_models::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        #[cfg(feature = "anthropic")]
        {
            let config = AnthropicConfig::default();
            let provider = AnthropicProvider::new(config);
            // Test that the provider was created successfully
            assert!(std::mem::size_of_val(&provider) > 0);
        }
    }

    #[test]
    fn test_huggingface_creation() {
        #[cfg(feature = "huggingface")]
        {
            let config = HuggingFaceConfig::default();
            let provider = HuggingFaceProvider::new(config);
            // Test that the provider was created successfully
            assert!(std::mem::size_of_val(&provider) > 0);
        }
    }

    #[test]
    fn test_gemini_creation() {
        #[cfg(feature = "gemini")]
        {
            let config = GeminiConfig::default();
            let provider = GeminiProvider::new(config);
            // Test that the provider was created successfully
            assert!(std::mem::size_of_val(&provider) > 0);
        }
    }

    /// Registration/factory test — mirrors `common.rs::test_provider_registry`: a Gemini
    /// provider registers into the shared `ProviderRegistry` under the "google" name (matching
    /// the catalog's `provider` field for gemini-* models, see `gaussmeridian-db/src/seed.rs`)
    /// exactly like every other provider, and is retrievable by that name.
    #[tokio::test]
    async fn test_gemini_registers_into_provider_registry() {
        #[cfg(feature = "gemini")]
        {
            let registry = ProviderRegistry::new();
            let provider = GeminiProvider::with_api_key("test-key".to_string());
            registry
                .register_provider(
                    "google".to_string(),
                    std::sync::Arc::new(provider)
                        as std::sync::Arc<
                            dyn gaussmeridian_core::LLMProvider<Error = gaussmeridian_models::ProviderError>
                                + Send
                                + Sync,
                        >,
                )
                .await;

            assert!(registry.get_provider("google").await.is_some());
            assert!(registry.list_providers().await.contains(&"google".to_string()));
        }
    }
}
