use crate::{
    error::MoaResult,
    models::{AgentResponse, MoaRequest},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

/// Token usage metrics tracker
#[derive(Debug)]
pub struct TokenMetricsTracker {
    /// Token usage by agent
    agent_tokens: Arc<RwLock<HashMap<String, AgentTokenMetrics>>>,
    /// Token usage by request
    request_tokens: Arc<RwLock<HashMap<String, RequestTokenMetrics>>>,
    /// Global token metrics
    global_metrics: Arc<RwLock<GlobalTokenMetrics>>,
}

/// Agent token usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTokenMetrics {
    /// Total input tokens
    pub input_tokens: u64,
    /// Total output tokens
    pub output_tokens: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Average tokens per request
    pub avg_tokens_per_request: f64,
    /// Token usage by category
    pub category_usage: HashMap<String, u64>,
}

/// Request token usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTokenMetrics {
    /// Total tokens used
    pub total_tokens: u64,
    /// Tokens by agent
    pub agent_tokens: HashMap<String, u64>,
    /// Tokens by phase
    pub phase_tokens: HashMap<String, u64>,
    /// Request duration in milliseconds
    pub duration_ms: u64,
    /// Token rate (tokens/second)
    pub token_rate: f64,
}

/// Global token usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTokenMetrics {
    /// Total tokens used
    pub total_tokens: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Average tokens per request
    pub avg_tokens_per_request: f64,
    /// Token usage by category
    pub category_usage: HashMap<String, u64>,
    /// Token usage by model
    pub model_usage: HashMap<String, u64>,
}

impl TokenMetricsTracker {
    /// Create a new token metrics tracker
    pub fn new() -> Self {
        Self {
            agent_tokens: Arc::new(RwLock::new(HashMap::new())),
            request_tokens: Arc::new(RwLock::new(HashMap::new())),
            global_metrics: Arc::new(RwLock::new(GlobalTokenMetrics {
                total_tokens: 0,
                total_requests: 0,
                avg_tokens_per_request: 0.0,
                category_usage: HashMap::new(),
                model_usage: HashMap::new(),
            })),
        }
    }

    /// Track token usage for an agent response
    pub async fn track_agent_response(
        &self,
        agent_id: &str,
        request: &MoaRequest,
        response: &AgentResponse,
        input_tokens: u64,
        output_tokens: u64,
        category: &str,
    ) -> MoaResult<()> {
        // Update agent metrics
        let mut agent_metrics = self.agent_tokens.write().await;
        let metrics = agent_metrics.entry(agent_id.to_string()).or_insert(AgentTokenMetrics {
            input_tokens: 0,
            output_tokens: 0,
            total_requests: 0,
            avg_tokens_per_request: 0.0,
            category_usage: HashMap::new(),
        });

        metrics.input_tokens += input_tokens;
        metrics.output_tokens += output_tokens;
        metrics.total_requests += 1;
        metrics.avg_tokens_per_request = (metrics.input_tokens + metrics.output_tokens) as f64
            / metrics.total_requests as f64;
        
        *metrics.category_usage.entry(category.to_string()).or_insert(0) += input_tokens + output_tokens;

        // Update request metrics
        let mut request_metrics = self.request_tokens.write().await;
        let req_metrics = request_metrics.entry(request.id.to_string()).or_insert(RequestTokenMetrics {
            total_tokens: 0,
            agent_tokens: HashMap::new(),
            phase_tokens: HashMap::new(),
            duration_ms: 0,
            token_rate: 0.0,
        });

        req_metrics.total_tokens += input_tokens + output_tokens;
        *req_metrics.agent_tokens.entry(agent_id.to_string()).or_insert(0) += input_tokens + output_tokens;
        *req_metrics.phase_tokens.entry(category.to_string()).or_insert(0) += input_tokens + output_tokens;

        // Update global metrics
        let mut global = self.global_metrics.write().await;
        global.total_tokens += input_tokens + output_tokens;
        global.total_requests += 1;
        global.avg_tokens_per_request = global.total_tokens as f64 / global.total_requests as f64;
        
        *global.category_usage.entry(category.to_string()).or_insert(0) += input_tokens + output_tokens;
        *global.model_usage.entry(response.agent_id.clone()).or_insert(0) += input_tokens + output_tokens;

        Ok(())
    }

    /// Get agent token metrics
    pub async fn get_agent_metrics(&self, agent_id: &str) -> Option<AgentTokenMetrics> {
        self.agent_tokens.read().await.get(agent_id).cloned()
    }

    /// Get request token metrics
    pub async fn get_request_metrics(&self, request_id: &str) -> Option<RequestTokenMetrics> {
        self.request_tokens.read().await.get(request_id).cloned()
    }

    /// Get global token metrics
    pub async fn get_global_metrics(&self) -> GlobalTokenMetrics {
        self.global_metrics.read().await.clone()
    }

    /// Update request duration
    pub async fn update_request_duration(
        &self,
        request_id: &str,
        duration_ms: u64,
    ) -> MoaResult<()> {
        let mut request_metrics = self.request_tokens.write().await;
        if let Some(metrics) = request_metrics.get_mut(request_id) {
            metrics.duration_ms = duration_ms;
            metrics.token_rate = metrics.total_tokens as f64 / (duration_ms as f64 / 1000.0);
        }
        Ok(())
    }

    /// Generate token usage report
    pub async fn generate_report(&self) -> TokenUsageReport {
        let global = self.global_metrics.read().await;
        let agents = self.agent_tokens.read().await;
        let requests = self.request_tokens.read().await;

        TokenUsageReport {
            total_tokens: global.total_tokens,
            total_requests: global.total_requests,
            avg_tokens_per_request: global.avg_tokens_per_request,
            category_breakdown: global.category_usage.clone(),
            model_breakdown: global.model_usage.clone(),
            agent_metrics: agents.clone(),
            request_metrics: requests.clone(),
        }
    }

    /// Reset metrics
    pub async fn reset(&self) -> MoaResult<()> {
        let mut agent_metrics = self.agent_tokens.write().await;
        let mut request_metrics = self.request_tokens.write().await;
        let mut global = self.global_metrics.write().await;

        agent_metrics.clear();
        request_metrics.clear();
        *global = GlobalTokenMetrics {
            total_tokens: 0,
            total_requests: 0,
            avg_tokens_per_request: 0.0,
            category_usage: HashMap::new(),
            model_usage: HashMap::new(),
        };

        Ok(())
    }
}

/// Token usage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageReport {
    /// Total tokens used
    pub total_tokens: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Average tokens per request
    pub avg_tokens_per_request: f64,
    /// Token usage by category
    pub category_breakdown: HashMap<String, u64>,
    /// Token usage by model
    pub model_breakdown: HashMap<String, u64>,
    /// Agent-specific metrics
    pub agent_metrics: HashMap<String, AgentTokenMetrics>,
    /// Request-specific metrics
    pub request_metrics: HashMap<String, RequestTokenMetrics>,
}

pub struct TokenMetrics {
    pub model_usage: Arc<RwLock<HashMap<String, usize>>>,
    pub request_metrics: Arc<RwLock<HashMap<String, RequestTokenMetrics>>>,
}

impl TokenMetrics {
    pub fn new() -> Self {
        Self {
            model_usage: Arc::new(RwLock::new(HashMap::new())),
            request_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_metrics(&self, response: &AgentResponse, input_tokens: usize, output_tokens: usize) {
        let mut global_model_usage = self.model_usage.write().await;
        *global_model_usage.entry(response.agent_id.clone()).or_insert(0) += input_tokens + output_tokens;

        let mut request_metrics_map = self.request_metrics.write().await;
        let total_tokens_for_response = (input_tokens + output_tokens) as u64;

        let metrics_entry = request_metrics_map
            .entry(response.id.clone())
            .or_insert_with(|| RequestTokenMetrics {
                total_tokens: 0,
                agent_tokens: HashMap::new(),
                phase_tokens: HashMap::new(),
                duration_ms: 0,
                token_rate: 0.0,
            });

        metrics_entry.total_tokens += total_tokens_for_response;
        *metrics_entry.agent_tokens.entry(response.agent_id.clone()).or_insert(0) += total_tokens_for_response;
    }

    pub async fn get_model_usage(&self, agent_id: &str) -> Option<usize> {
        self.model_usage.read().await.get(agent_id).copied()
    }

    pub async fn get_request_metrics(&self, request_id: &str) -> Option<RequestTokenMetrics> {
        self.request_metrics.read().await.get(request_id).cloned()
    }
} 