//! Cache-related types and structures

use gaussmeridian_models::{ChatCompletionResponse, CompletionResponse, EmbeddingResponse};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CacheKey {
    pub provider: String,
    pub model: String,
    pub request_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheValue {
    ChatCompletion(ChatCompletionResponse),
    Completion(CompletionResponse),
    Embedding(EmbeddingResponse),
}
