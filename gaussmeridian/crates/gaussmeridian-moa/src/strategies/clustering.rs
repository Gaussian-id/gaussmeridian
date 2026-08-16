use crate::{
    error::{MoaResult, MoaError},
    models::{AgentResponse, MoaRequest, MoaResponse, ResponseMetrics},
    strategies::Strategy,
};
use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug)]
pub enum ClusteringError {
    InsufficientData(String),
    EmbeddingError(String),
    KMeansError(String),
    Timeout(String),
}

impl std::fmt::Display for ClusteringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusteringError::InsufficientData(msg) => write!(f, "Insufficient data: {}", msg),
            ClusteringError::EmbeddingError(msg) => write!(f, "Embedding error: {}", msg),
            ClusteringError::KMeansError(msg) => write!(f, "K-means error: {}", msg),
            ClusteringError::Timeout(msg) => write!(f, "Timeout: {}", msg),
        }
    }
}

impl std::error::Error for ClusteringError {}

#[derive(Default)]
pub struct ClusteringStrategy;

#[async_trait]
impl Strategy for ClusteringStrategy {
    async fn process_responses(
        &self,
        responses: Vec<AgentResponse>,
        _request: &MoaRequest,
    ) -> MoaResult<MoaResponse> {
        if responses.is_empty() {
            return Err(MoaError::Strategy {
                message: "No responses to process".to_string(),
                source: None
            });
        }

        // Find best response based on confidence
        let best_response = responses
            .into_iter()
            .max_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap())
            .ok_or_else(|| MoaError::Strategy {
                message: "No response met confidence threshold".to_string(),
                source: None
            })?;

        let metrics = best_response.metrics.clone();
        Ok(MoaResponse {
            id: Uuid::new_v4(),
            content: best_response.content.clone(),
            confidence: best_response.confidence as f32,
            agent_responses: vec![best_response],
            timestamp: Utc::now(),
            metrics: ResponseMetrics {
                latency_ms: metrics.latency_ms,
                tokens_used: metrics.tokens_used,
                prompt_tokens: metrics.prompt_tokens,
                completion_tokens: metrics.completion_tokens,
            },
        })
    }

    async fn warmup(&self) -> MoaResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        Ok(())
    }
} 