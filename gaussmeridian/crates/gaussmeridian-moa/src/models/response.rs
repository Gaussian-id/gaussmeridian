use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::request::MoaRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Response ID
    pub id: String,
    /// Agent ID
    pub agent_id: String,
    /// Original request
    pub request: MoaRequest,
    /// Response content
    pub content: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Response timestamp
    pub timestamp: DateTime<Utc>,
    /// Response metrics
    pub metrics: ResponseMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoaResponse {
    pub id: Uuid,
    pub content: String,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
    pub agent_responses: Vec<AgentResponse>,
    pub metrics: ResponseMetrics,
}

impl MoaResponse {
    pub fn new(content: String, agent_responses: Vec<AgentResponse>) -> Self {
        let confidence = if !agent_responses.is_empty() {
            agent_responses.iter()
                .map(|r| r.confidence as f32)
                .sum::<f32>() / agent_responses.len() as f32
        } else {
            0.0
        };
        
        Self {
            id: Uuid::new_v4(),
            content,
            confidence,
            timestamp: Utc::now(),
            agent_responses,
            metrics: ResponseMetrics::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseMetrics {
    pub latency_ms: u64,
    pub tokens_used: u32,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}