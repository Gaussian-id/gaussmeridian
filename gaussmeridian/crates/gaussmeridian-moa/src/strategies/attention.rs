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
pub struct AttentionConfig {
    pub attention_heads: u32,
    pub key_size: u32,
    pub value_size: u32,
    pub min_attention_score: f32,
    pub temperature: f32,
}

impl Default for AttentionConfig {
    fn default() -> Self {
        Self {
            attention_heads: 4,
            key_size: 64,
            value_size: 64,
            min_attention_score: 0.1,
            temperature: 1.0,
        }
    }
}

#[derive(Debug)]
pub struct AttentionStrategy {
    config: AttentionConfig,
}

impl AttentionStrategy {
    pub fn new(config: Option<AttentionConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
        }
    }

    fn compute_attention_scores(&self, responses: &[AgentResponse]) -> Vec<f64> {
        let mut scores = Vec::with_capacity(responses.len());
        let total_confidence: f64 = responses.iter().map(|r| r.confidence).sum();

        for response in responses {
            let attention_score = response.confidence / total_confidence;
            scores.push(attention_score);
        }

        // Apply temperature scaling
        if self.config.temperature != 1.0 {
            for score in &mut scores {
                *score = (*score / self.config.temperature as f64).exp();
            }
            let sum: f64 = scores.iter().sum();
            for score in &mut scores {
                *score /= sum;
            }
        }

        scores
    }
}

#[async_trait]
impl Strategy for AttentionStrategy {
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

        // Compute attention scores
        let attention_scores = self.compute_attention_scores(&responses);

        // Find best response based on attention scores
        let best_response = responses
            .into_iter()
            .zip(attention_scores)
            .max_by(|(a, a_score), (b, b_score)| {
                let a_total = a.confidence * a_score;
                let b_total = b.confidence * b_score;
                b_total.partial_cmp(&a_total).unwrap()
            })
            .map(|(response, _)| response)
            .ok_or_else(|| MoaError::Strategy {
                message: "No response met attention threshold".to_string(),
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
        debug!("Warming up attention strategy");
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        debug!("Shutting down attention strategy");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_attention_strategy() {
        let config = AttentionConfig {
            attention_heads: 2,
            key_size: 32,
            value_size: 32,
            min_attention_score: 0.2,
            temperature: 0.8,
        };
        let strategy = AttentionStrategy::new(Some(config));

        let request = MoaRequest::new("test query".to_string(), None);
        let responses = vec![
            AgentResponse {
                id: Uuid::new_v4().to_string(),
                agent_id: "agent1".to_string(),
                request: request.clone(),
                content: "Response 1".to_string(),
                confidence: 0.8,
                timestamp: Utc::now(),
                metrics: ResponseMetrics {
                    latency_ms: 100,
                    tokens_used: 50,
                    prompt_tokens: 20,
                    completion_tokens: 30,
                },
            },
            AgentResponse {
                id: Uuid::new_v4().to_string(),
                agent_id: "agent2".to_string(),
                request: request.clone(),
                content: "Response 2".to_string(),
                confidence: 0.9,
                timestamp: Utc::now(),
                metrics: ResponseMetrics {
                    latency_ms: 100,
                    tokens_used: 50,
                    prompt_tokens: 20,
                    completion_tokens: 30,
                },
            },
        ];

        let result = strategy.process_responses(responses, &request).await;
        assert!(result.is_ok());
    }
} 