use crate::{
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest, MoaResponse, ResponseMetrics},
    strategies::Strategy,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseConfig {
    pub k: usize,
    pub confidence_threshold: f32,
}

impl Default for SparseConfig {
    fn default() -> Self {
        Self {
            k: 2,
            confidence_threshold: 0.7,
        }
    }
}

#[derive(Default)]
pub struct SparseStrategy {
    config: SparseConfig,
}

#[async_trait]
impl Strategy for SparseStrategy {
    async fn process_responses(
        &self,
        mut responses: Vec<AgentResponse>,
        _request: &MoaRequest,
    ) -> MoaResult<MoaResponse> {
        if responses.is_empty() {
            return Err(MoaError::Strategy {
                message: "No responses to process".to_string(),
                source: None
            });
        }

        // Sort by confidence
        responses.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        // Take top k responses above threshold
        let selected_responses: Vec<_> = responses
            .into_iter()
            .filter(|r| r.confidence >= self.config.confidence_threshold as f64)
            .take(self.config.k)
            .collect();

        // Select best response
        let best_response = selected_responses
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
        debug!("Warming up sparse strategy");
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        debug!("Shutting down sparse strategy");
        Ok(())
    }
} 