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
pub struct AdaptiveConfig {
    pub learning_rate: f32,
    pub exploration_rate: f32,
    pub min_samples: usize,
    pub max_history: usize,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            exploration_rate: 0.2,
            min_samples: 10,
            max_history: 1000,
        }
    }
}

#[derive(Default)]
pub struct AdaptiveStrategy {
    config: AdaptiveConfig,
    performance_history: HashMap<String, Vec<f32>>,
}

impl AdaptiveStrategy {
    fn update_performance(&mut self, agent_id: &str, performance: f32) {
        let history = self.performance_history
            .entry(agent_id.to_string())
            .or_insert_with(Vec::new);

        history.push(performance);

        // Keep history size bounded
        if history.len() > self.config.max_history {
            history.remove(0);
        }
    }

    fn get_agent_score(&self, agent_id: &str) -> f32 {
        let empty_vec = Vec::new();
        let history = self.performance_history
            .get(agent_id)
            .unwrap_or(&empty_vec);

        if history.len() < self.config.min_samples {
            return 0.5; // Default score for agents with insufficient history
        }

        let recent_performance: f32 = history.iter().sum::<f32>() / history.len() as f32;
        let exploration_bonus = if history.len() < self.config.min_samples * 2 {
            self.config.exploration_rate
        } else {
            0.0
        };

        recent_performance + exploration_bonus
    }
}

#[async_trait]
impl Strategy for AdaptiveStrategy {
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

        // Find best response based on confidence and historical performance
        let best_response = responses
            .into_iter()
            .max_by(|a, b| {
                let a_score = a.confidence * self.get_agent_score(&a.agent_id) as f64;
                let b_score = b.confidence * self.get_agent_score(&b.agent_id) as f64;
                b_score.partial_cmp(&a_score).unwrap()
            })
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
        debug!("Warming up adaptive strategy");
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        debug!("Shutting down adaptive strategy");
        Ok(())
    }
} 