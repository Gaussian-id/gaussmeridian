//! Shared data models for GaussMeridian
//!
//! This crate contains all the data structures used across the GaussMeridian
//! ecosystem for requests, responses, and internal state management.

pub mod error;
pub mod request;
pub mod response;
pub mod shared;

pub use error::*;
pub use request::*;
pub use response::*;
pub use shared::*;

/// Re-export commonly used types
pub mod prelude {
    pub use super::{
        ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, CompletionChunk,
        CompletionRequest, CompletionResponse, Content, ContentPart, CostInfo, EmbeddingRequest,
        EmbeddingResponse, GaussMeridianError, Message, Model, ModelCapabilities, ModelInfo,
        ProviderCapabilities, ProviderError, ProviderMetadata, Role,
    };
}
