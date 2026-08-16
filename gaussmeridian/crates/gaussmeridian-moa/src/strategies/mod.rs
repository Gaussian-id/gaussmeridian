use crate::{
    agents::Agent,
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest, MoaResponse},
};
use async_trait::async_trait;

pub mod attention;
pub mod clustering;
pub mod debate;
pub mod roles;
pub mod routing;
pub mod standard;
pub mod sparse;
pub mod self_moa;
pub mod collaborative;
pub mod adaptive;
pub mod persistence;

pub use attention::AttentionStrategy;
pub use clustering::ClusteringStrategy;
pub use debate::DebateStrategy;
pub use roles::RolesStrategy;
pub use routing::RoutingStrategy;
pub use standard::StandardStrategy;
pub use sparse::SparseStrategy;
pub use self_moa::SelfMoaStrategy;
pub use collaborative::CollaborativeStrategy;
pub use adaptive::AdaptiveStrategy;

#[async_trait]
pub trait Strategy: Send + Sync {
    /// Process responses from multiple agents and return the best one
    async fn process_responses(&self, responses: Vec<AgentResponse>, _request: &MoaRequest) -> MoaResult<MoaResponse>;

    /// Warm up the strategy for optimal performance
    async fn warmup(&self) -> MoaResult<()>;

    /// Gracefully shutdown the strategy
    async fn shutdown(&self) -> MoaResult<()>;

    /// Check strategy health
    async fn health_check(&self) -> MoaResult<bool> {
        Ok(true)
    }
}

/// Adaptive mixture strategy that learns from agent performance
pub struct AdaptiveMoaStrategy {
    /// Learning rate for weight updates
    learning_rate: f64,
    /// Temperature for exploration
    temperature: f64,
    /// Performance history window size
    history_window: usize,
    /// Maximum concurrent requests
    max_concurrent: usize,
}

impl AdaptiveMoaStrategy {
    pub fn new(
        learning_rate: f64,
        temperature: f64,
        history_window: usize,
        max_concurrent: usize,
    ) -> Self {
        Self {
            learning_rate,
            temperature,
            history_window,
            max_concurrent,
        }
    }

    async fn update_weights(
        &self,
        agents: &[Box<dyn Agent>],
        responses: &[AgentResponse],
    ) -> Vec<f64> {
        let mut weights = vec![1.0; agents.len()];
        let total_confidence: f64 = responses.iter().map(|r| r.confidence).sum();

        for (i, response) in responses.iter().enumerate() {
            weights[i] *= (response.confidence / total_confidence) * self.learning_rate;
        }

        // Normalize weights
        let sum: f64 = weights.iter().sum();
        weights.iter_mut().for_each(|w| *w /= sum);

        weights
    }
}

#[async_trait]
impl Strategy for AdaptiveMoaStrategy {
    async fn process_responses(&self, responses: Vec<AgentResponse>, _request: &MoaRequest) -> MoaResult<MoaResponse> {
        if responses.is_empty() {
            return Err(MoaError::Strategy {
                message: "No responses to process".to_string(),
                source: None
            });
        }

        let best_response = responses.into_iter()
            .max_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap())
            .ok_or_else(|| MoaError::Strategy {
                message: "No response met confidence threshold".to_string(),
                source: None
            })?;

        Ok(MoaResponse {
            id: uuid::Uuid::new_v4(),
            content: best_response.content.clone(),
            confidence: best_response.confidence as f32,
            agent_responses: vec![best_response],
            timestamp: chrono::Utc::now(),
            metrics: crate::models::ResponseMetrics::default(),
        })
    }

    async fn warmup(&self) -> MoaResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        Ok(())
    }
}

pub fn normalize_weights(weights: &mut [f64]) {
    let sum: f64 = weights.iter().sum();
    if sum > 0.0 {
        weights.iter_mut().for_each(|w| *w /= sum);
    }
}

/// Run a strategy
pub fn create_strategy(name: &str) -> MoaResult<Box<dyn Strategy>> {
    match name {
        "standard" => Ok(Box::new(StandardStrategy::default())),
        "sparse" => Ok(Box::new(SparseStrategy::default())),
        "self_moa" => Ok(Box::new(SelfMoaStrategy::default())),
        "debate" => Ok(Box::new(DebateStrategy::new(None))),
        "attention" => Ok(Box::new(AttentionStrategy::new(None))),
        "collaborative" => Ok(Box::new(CollaborativeStrategy::default())),
        "adaptive" => Ok(Box::new(AdaptiveStrategy::default())),
        _ => Err(MoaError::Strategy {
            message: format!("Strategy {} not found", name),
            source: None
        })
    }
}