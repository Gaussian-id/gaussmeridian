use crate::{
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest, MoaResponse, ResponseMetrics},
    strategies::Strategy,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborativeConfig {
    pub min_participants: usize,
    pub max_participants: usize,
    pub consensus_threshold: f32,
    pub max_rounds: usize,
}

impl Default for CollaborativeConfig {
    fn default() -> Self {
        Self {
            min_participants: 2,
            max_participants: 5,
            consensus_threshold: 0.7,
            max_rounds: 3,
        }
    }
}

#[derive(Default)]
pub struct CollaborativeStrategy {
    config: CollaborativeConfig,
}

impl CollaborativeStrategy {
    fn calculate_consensus(&self, responses: &[AgentResponse]) -> f64 {
        if responses.is_empty() {
            return 0.0;
        }

        let total_confidence: f64 = responses.iter().map(|r| r.confidence).sum();
        let max_confidence = responses.iter().map(|r| r.confidence).fold(0.0, f64::max);

        max_confidence / total_confidence
    }

    fn calculate_similarity(content1: &str, content2: &str) -> f32 {
        // Simple similarity calculation based on content length
        let len1 = content1.len();
        let len2 = content2.len();
        let max_len = len1.max(len2) as f32;
        let min_len = len1.min(len2) as f32;
        min_len / max_len
    }
}

#[async_trait]
impl Strategy for CollaborativeStrategy {
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

        // Calculate similarity scores
        let mut similarity_scores = HashMap::new();
        for (i, response1) in responses.iter().enumerate() {
            let mut score = 0.0;
            for (j, response2) in responses.iter().enumerate() {
                if i != j {
                    score += Self::calculate_similarity(&response1.content, &response2.content);
                }
            }
            similarity_scores.insert(i, score / (responses.len() - 1) as f32);
        }

        // Select best response based on consensus, similarity, and confidence
        let best_response = responses
            .into_iter()
            .enumerate()
            .max_by(|(i, a), (j, b)| {
                let a_score = a.confidence * consensus * similarity_scores[i] as f64;
                let b_score = b.confidence * consensus * similarity_scores[j] as f64;
                b_score.partial_cmp(&a_score).unwrap()
            })
            .map(|(_, response)| response)
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
        debug!("Warming up collaborative strategy");
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        debug!("Shutting down collaborative strategy");
        Ok(())
    }
} 