use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

mod request;
mod response;


/// Request metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestMetadata {
    /// Agent ID
    pub agent_id: Option<String>,
    /// Model used
    pub model: Option<String>,
    /// Temperature for sampling
    pub temperature: Option<f32>,
    /// Maximum response tokens
    pub max_tokens: Option<u32>,
    /// Custom parameters
    pub custom: std::collections::HashMap<String, String>,
}

impl RequestMetadata {
    pub fn insert(&mut self, key: String, value: String) {
        self.custom.insert(key, value);
    }
}

/// Response from an agent
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

/// Response metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseMetrics {
    /// Generation time in milliseconds
    pub latency_ms: u64,
    /// Token count
    pub tokens_used: u32,
    /// Prompt tokens
    pub prompt_tokens: u32,
    /// Completion tokens
    pub completion_tokens: u32,
}

/// MOA response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoaResponse {
    /// Response ID
    pub id: Uuid,
    /// Response content
    pub content: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Agent responses
    pub agent_responses: Vec<AgentResponse>,
    /// Response timestamp
    pub timestamp: DateTime<Utc>,
    /// Response metrics
    pub metrics: ResponseMetrics,
}

impl MoaResponse {
    pub fn into_agent_response(self) -> AgentResponse {
        AgentResponse {
            id: self.id.to_string(),
            agent_id: self.agent_responses[0].agent_id.clone(),
            request: self.agent_responses[0].request.clone(),
            content: self.content,
            confidence: self.confidence as f64,
            timestamp: self.timestamp,
            metrics: self.metrics,
        }
    }
}

/// Agent role
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    /// Primary agent
    Primary,
    /// Secondary agent
    Secondary,
    /// Validator agent
    Validator,
    /// Critic agent
    Critic,
    /// Custom agent
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoaRequest {
    pub id: String,
    pub query: String,
    pub context: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl Default for MoaRequest {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            query: String::new(),
            context: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

impl MoaRequest {
    pub fn new(query: String, context: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            query,
            context,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

impl AgentResponse {
    pub fn new(
        agent_id: String,
        request: MoaRequest,
        content: String,
        confidence: f64,
        metrics: ResponseMetrics,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent_id,
            request,
            content,
            confidence,
            timestamp: Utc::now(),
            metrics,
        }
    }
}

/// Health status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }
}

/// Agent health status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl AgentHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, AgentHealth::Healthy)
    }
}

/// Detailed health status for components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedHealthStatus {
    pub resources: ComponentHealthStatus,
    pub agents: DetailedAgentHealth,
    pub strategies: ComponentHealthStatus,
    pub processor: ComponentHealthStatus,
    pub timestamp: DateTime<Utc>,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealthStatus {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Detailed agent health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedAgentHealth {
    pub agent_statuses: HashMap<String, (AgentHealth, crate::agents::AgentMetrics)>,
    pub total_agents: usize,
    pub timestamp: DateTime<Utc>,
}

impl DetailedAgentHealth {
    pub fn status(&self) -> AgentHealth {
        let unhealthy_count = self.agent_statuses.values()
            .filter(|(health, _)| *health == AgentHealth::Unhealthy)
            .count();
        
        if unhealthy_count == 0 {
            AgentHealth::Healthy
        } else if unhealthy_count < self.agent_statuses.len() {
            AgentHealth::Degraded
        } else {
            AgentHealth::Unhealthy
        }
    }
}