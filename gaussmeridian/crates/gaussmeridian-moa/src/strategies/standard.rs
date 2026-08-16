use crate::{
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest, MoaResponse, ResponseMetrics},
    strategies::Strategy,
};
use async_trait::async_trait;
use tracing::debug;
use uuid::Uuid;
use chrono::Utc;

#[derive(Default)]
pub struct StandardStrategy;

#[async_trait]
impl Strategy for StandardStrategy {
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
        debug!("Warming up standard strategy");
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        debug!("Shutting down standard strategy");
        Ok(())
    }
} 