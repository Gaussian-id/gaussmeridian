use crate::{
    error::MoaResult,
    models::{AgentResponse, MoaRequest, MoaResponse, ResponseMetrics},
    strategies::Strategy,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfMoaConfig {
    pub samples: usize,
    pub max_rounds: usize,
    pub confidence_threshold: f32,
}

impl Default for SelfMoaConfig {
    fn default() -> Self {
        Self {
            samples: 3,
            max_rounds: 3,
            confidence_threshold: 0.8,
        }
    }
}

#[derive(Default)]
pub struct SelfMoaStrategy {
    config: SelfMoaConfig,
}

#[async_trait]
impl Strategy for SelfMoaStrategy {
    async fn process_responses(
        &self,
        responses: Vec<AgentResponse>,
        _request: &MoaRequest,
    ) -> MoaResult<MoaResponse> {
        let mut current_responses = responses;
        let mut current_round = 0;

        while current_round < self.config.max_rounds {
            // Sample responses
            let samples: Vec<_> = current_responses
                .iter()
                .take(self.config.samples)
                .cloned()
                .collect();

            // Find best response in samples
            let best_sample = samples
                .into_iter()
                .max_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap())
                .ok_or_else(|| crate::error::MoaError::Strategy {
                    message: "No responses to process".to_string(),
                    source: None,
                })?;

            // Check if confidence is high enough
            if best_sample.confidence >= self.config.confidence_threshold as f64 {
                let metrics = best_sample.metrics.clone();
                return Ok(MoaResponse {
                    id: Uuid::new_v4(),
                    content: best_sample.content.clone(),
                    confidence: best_sample.confidence as f32,
                    agent_responses: vec![best_sample],
                    timestamp: Utc::now(),
                    metrics: ResponseMetrics {
                        latency_ms: metrics.latency_ms,
                        tokens_used: metrics.tokens_used,
                        prompt_tokens: metrics.prompt_tokens,
                        completion_tokens: metrics.completion_tokens,
                    },
                });
            }

            // Update responses for next round
            current_responses = current_responses
                .into_iter()
                .filter(|r| r.confidence > best_sample.confidence * 0.8)
                .collect();

            current_round += 1;
        }

        // Return best response after max rounds
        let best_response = current_responses
            .into_iter()
            .max_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap())
            .ok_or_else(|| crate::error::MoaError::Strategy {
                message: "No response met confidence threshold".to_string(),
                source: None,
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
        debug!("Warming up self-moa strategy");
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        debug!("Shutting down self-moa strategy");
        Ok(())
    }
} 