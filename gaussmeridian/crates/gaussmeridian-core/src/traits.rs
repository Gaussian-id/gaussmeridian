//! Core traits for LLM providers

use async_trait::async_trait;
use futures::Stream;
use gaussmeridian_models::*;
use std::pin::Pin;

#[async_trait]
pub trait LLMProvider: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Self::Error>;

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    >;

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, Self::Error>;

    async fn completion_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk, Self::Error>> + Send>>, Self::Error>;

    async fn embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, Self::Error>;

    async fn list_models(&self) -> Result<Vec<Model>, Self::Error>;

    fn metadata(&self) -> ProviderMetadata;

    async fn health_check(&self) -> Result<(), Self::Error>;

    fn capabilities(&self) -> ProviderCapabilities;

    async fn get_cost_info(&self, model: &str) -> Result<CostInfo, Self::Error>;

    async fn supports_model(&self, model: &str) -> bool;

    fn get_config(&self) -> ProviderConfig;
}
