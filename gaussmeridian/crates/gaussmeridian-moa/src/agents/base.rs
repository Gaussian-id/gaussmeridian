use crate::{
    agents::{Agent, metrics::AgentMetrics},
    config::AgentConfig,
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest},
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct BaseAgent {
    id: String,
    name: String,
    description: String,
    capabilities: Vec<String>,
    config: AgentConfig,
    metrics: Arc<RwLock<AgentMetrics>>,
}

impl BaseAgent {
    pub fn new(
        id: String,
        name: String,
        description: String,
        capabilities: Vec<String>,
        config: AgentConfig,
    ) -> Self {
        Self {
            id,
            name,
            description,
            capabilities,
            config,
            metrics: Arc::new(RwLock::new(AgentMetrics::default())),
        }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    pub fn get_capabilities(&self) -> &[String] {
        &self.capabilities
    }

    pub fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    pub fn get_metrics(&self) -> AgentMetrics {
        self.metrics.try_read().unwrap().clone()
    }

    pub fn update_config(&mut self, config: AgentConfig) -> MoaResult<()> {
        self.config = config;
        Ok(())
    }

    pub fn reset(&mut self) -> MoaResult<()> {
        *self.metrics.try_write().unwrap() = AgentMetrics::default();
        Ok(())
    }

    pub async fn record_request_outcome(&self, duration: std::time::Duration, confidence: f64, success: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        let new_latency = duration.as_millis() as f64;
        let request_count_for_latency = metrics.successful_requests;

        if request_count_for_latency == 0 {
            metrics.avg_latency_ms = 0.0;
        } else if request_count_for_latency == 1 && success {
             metrics.avg_latency_ms = new_latency;
        } else if success {
            let current_total_latency = metrics.avg_latency_ms * (request_count_for_latency - 1) as f64;
            metrics.avg_latency_ms = (current_total_latency + new_latency) / request_count_for_latency as f64;
        }

        if metrics.successful_requests == 0 {
             metrics.avg_confidence = 0.0;
        } else if metrics.successful_requests == 1 && success {
            metrics.avg_confidence = confidence as f32;
        } else if success {
            let current_total_confidence_f64 = metrics.avg_confidence as f64 * (metrics.successful_requests - 1) as f64;
            metrics.avg_confidence = ((current_total_confidence_f64 + confidence) / metrics.successful_requests as f64) as f32;
        }
        
        if metrics.total_requests > 0 {
            metrics.success_rate = (metrics.successful_requests as f64 / metrics.total_requests as f64) as f32;
        } else {
            metrics.success_rate = 0.0;
        }
    }
}

#[async_trait]
impl Agent for BaseAgent {
    fn get_id(&self) -> &str {
        BaseAgent::get_id(self)
    }

    fn get_name(&self) -> &str {
        BaseAgent::get_name(self)
    }

    fn get_description(&self) -> &str {
        BaseAgent::get_description(self)
    }

    fn get_capabilities(&self) -> &[String] {
        BaseAgent::get_capabilities(self)
    }

    fn get_config(&self) -> &AgentConfig {
        BaseAgent::get_config(self)
    }

    async fn process_request(&self, _request: &MoaRequest) -> MoaResult<AgentResponse> {
        Err(MoaError::agent( 
            "Base agent cannot process requests directly".to_string(),
            self.id.clone(),
            None::<Box<dyn std::error::Error + Send + Sync>> 
        ))
    }

    fn update_config(&mut self, config: AgentConfig) -> MoaResult<()> {
        BaseAgent::update_config(self, config)
    }

    fn get_metrics(&self) -> AgentMetrics {
        BaseAgent::get_metrics(self)
    }

    fn reset(&mut self) -> MoaResult<()> {
        BaseAgent::reset(self)
    }
}