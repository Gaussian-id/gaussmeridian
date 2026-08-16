use crate::{
    config::{AgentConfig, AgentRole},
    error::MoaResult,
    models::{AgentResponse, MoaRequest, ResponseMetrics},
    providers::ChatProvider,
    agents::{Agent, BaseAgent, AgentMetrics},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Instant};
use tokio::sync::RwLock;
use dashmap::DashMap;
use uuid::Uuid;
use chrono::Utc;

/// LLM provider types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmProvider {
    OpenAI {
        model: String,
        temperature: f32,
        max_tokens: usize,
    },
    Anthropic {
        model: String,
        temperature: f32,
        max_tokens: usize,
    },
    Local {
        model_path: String,
        temperature: f32,
    },
    Custom {
        endpoint: String,
        parameters: serde_json::Value,
    },
}

/// LLM agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAgentConfig {
    /// Provider configuration
    pub provider: LlmProvider,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Response format
    pub response_format: Option<String>,
    /// Timeout in seconds
    pub timeout_secs: u64,
    /// Retry configuration
    pub retries: Option<RetryConfig>,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: usize,
    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum retry delay in milliseconds
    pub max_delay_ms: u64,
    /// Retry backoff factor
    pub backoff_factor: f32,
}

/// LLM-based agent implementation
#[derive(Debug)]
pub struct LlmAgent {
    /// Base agent implementation
    base: BaseAgent,
    /// LLM configuration
    config: LlmAgentConfig,
    /// Provider the agent calls (injected — see `providers::ChatProvider`).
    provider: Arc<dyn ChatProvider>,
    /// Response cache
    cache: Arc<DashMap<String, AgentResponse>>,
    /// Load factor
    load_factor: Arc<RwLock<f32>>,
    /// Performance score
    performance_score: Arc<RwLock<f32>>,
}

impl LlmAgent {
    pub fn new(
        id: String,
        role: AgentRole,
        config: LlmAgentConfig,
        provider: Arc<dyn ChatProvider>,
    ) -> Self {
        Self {
            base: BaseAgent::new(
                id.clone(),
                "LLM Agent".to_string(),
                "An agent that uses LLM models to process requests".to_string(),
                Vec::new(),
                AgentConfig {
                    name: id.clone(),
                    agent_type: crate::config::AgentType::LLM,
                    role: role.clone(),
                    capabilities: Vec::new(),
                    config: serde_json::to_value(config.clone()).unwrap_or_default(),
                    max_retries: config.retries.as_ref().map_or(3u32, |r| r.max_retries as u32),
                    timeout_secs: config.timeout_secs,
                }
            ),
            config,
            provider,
            cache: Arc::new(DashMap::new()),
            load_factor: Arc::new(RwLock::new(0.0)),
            performance_score: Arc::new(RwLock::new(0.5)),
        }
    }


    /// Generate prompt for the model
    fn generate_prompt(&self, request: &MoaRequest) -> String {
        let mut prompt = String::new();

        // Add system prompt if available
        if let Some(system) = &self.config.system_prompt {
            prompt.push_str(system);
            prompt.push_str("\n\n");
        }

        // Add context if available
        if let Some(context) = &request.context {
            prompt.push_str("Context:\n");
            prompt.push_str(context);
            prompt.push_str("\n\n");
        }

        // Add query
        prompt.push_str("Query:\n");
        prompt.push_str(&request.query);

        // Add response format if available
        if let Some(format) = &self.config.response_format {
            prompt.push_str("\n\nPlease format your response as follows:\n");
            prompt.push_str(format);
        }

        prompt
    }

    /// Extract `(model, temperature, max_tokens)` from the provider config. The actual call goes
    /// through the injected `ChatProvider`, which decides how to reach the model.
    fn call_params(&self) -> (String, f32, usize) {
        match &self.config.provider {
            LlmProvider::OpenAI { model, temperature, max_tokens }
            | LlmProvider::Anthropic { model, temperature, max_tokens } => {
                (model.clone(), *temperature, *max_tokens)
            }
            LlmProvider::Local { model_path, temperature } => {
                (model_path.clone(), *temperature, 2048)
            }
            LlmProvider::Custom { endpoint: _, parameters } => {
                let model = parameters.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o-mini").to_string();
                let temperature = parameters.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = parameters.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(2048) as usize;
                (model, temperature, max_tokens)
            }
        }
    }

    /// Estimate response confidence
    fn estimate_confidence(&self, response: &str) -> f32 {
        // Implement confidence estimation based on response characteristics
        // This is a simple example - you should implement more sophisticated methods
        let length = response.len();
        let words = response.split_whitespace().count();
        
        if length == 0 || words == 0 {
            return 0.0;
        }

        let avg_word_length = length as f32 / words as f32;
        let normalized_length = (words as f32 / 100.0).min(1.0);
        let length_score = normalized_length * 0.7;
        let complexity_score = (avg_word_length / 10.0).min(1.0) * 0.3;

        length_score + complexity_score
    }

}

#[async_trait]
impl Agent for LlmAgent {
    fn get_id(&self) -> &str {
        self.base.get_id()
    }

    fn get_name(&self) -> &str {
        "LLM Agent"
    }

    fn get_description(&self) -> &str {
        "An agent that uses LLM models to process requests"
    }

    fn get_capabilities(&self) -> &[String] {
        self.base.get_capabilities()
    }

    fn get_config(&self) -> &AgentConfig {
        self.base.get_config()
    }

    async fn process_request(&self, request: &MoaRequest) -> MoaResult<AgentResponse> {
        let start = Instant::now();
        let prompt = self.generate_prompt(request);

        let (model, temperature, max_tokens) = self.call_params();
        let content = self
            .provider
            .complete(&model, &prompt, temperature, max_tokens)
            .await?;

        let confidence = self.estimate_confidence(&content);
        let response = AgentResponse {
            id: Uuid::new_v4().to_string(),
            agent_id: self.get_id().to_string(),
            request: request.clone(),
            content,
            confidence: confidence as f64,
            timestamp: Utc::now(),
            metrics: ResponseMetrics::default(),
        };

        // Update metrics
        self.base.record_request_outcome(start.elapsed(), response.confidence, true).await;

        Ok(response)
    }

    fn update_config(&mut self, config: AgentConfig) -> MoaResult<()> {
        self.base.update_config(config)
    }

    fn get_metrics(&self) -> AgentMetrics {
        self.base.get_metrics()
    }

    fn reset(&mut self) -> MoaResult<()> {
        self.cache.clear();
        if let Ok(mut load_factor) = self.load_factor.try_write() {
            *load_factor = 0.0;
        }
        if let Ok(mut performance_score) = self.performance_score.try_write() {
            *performance_score = 0.5;
        }
        self.base.reset()
    }
}