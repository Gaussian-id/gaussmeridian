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
pub struct DebateConfig {
    pub max_rounds: u32,
    pub min_consensus: f32,
    pub max_participants: u32,
    pub timeout_secs: u32,
}

impl Default for DebateConfig {
    fn default() -> Self {
        Self {
            max_rounds: 3,
            min_consensus: 0.7,
            max_participants: 5,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug)]
pub struct DebateStrategy {
    config: DebateConfig,
}

impl DebateStrategy {
    pub fn new(config: Option<DebateConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
        }
    }

    fn calculate_consensus(&self, responses: &[AgentResponse]) -> f64 {
        if responses.is_empty() {
            return 0.0;
        }

        let total_confidence: f64 = responses.iter().map(|r| r.confidence).sum();
        let max_confidence = responses.iter().map(|r| r.confidence).fold(0.0, f64::max);

        max_confidence / total_confidence
    }
}

#[async_trait]
impl Strategy for DebateStrategy {
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

        // Calculate consensus among responses
        let consensus = self.calculate_consensus(&responses);

        // Select best response based on consensus and confidence
        let best_response = responses
            .into_iter()
            .max_by(|a, b| {
                let a_score = a.confidence * consensus;
                let b_score = b.confidence * consensus;
                b_score.partial_cmp(&a_score).unwrap()
            })
            .ok_or_else(|| MoaError::Strategy {
                message: "No response met consensus threshold".to_string(),
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
        debug!("Warming up debate strategy");
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        debug!("Shutting down debate strategy");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_debate_strategy() {
        let config = DebateConfig {
            max_rounds: 2,
            min_consensus: 0.6,
            max_participants: 3,
            timeout_secs: 10,
        };
        let strategy = DebateStrategy::new(Some(config));

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