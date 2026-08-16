use crate::{
    config::MoaConfig,
    error::{MoaError, MoaResult},
    models::{MoaRequest, MoaResponse, AgentResponse, HealthStatus},
    agents::Agent,
};
use super::{AgentManager, ResourceManager, StrategyManager};
use std::{sync::Arc, time::Instant};
use tokio::{sync::RwLock, time::timeout};
use tracing::{debug, info, warn, error, instrument};
use futures::{stream::FuturesUnordered, StreamExt};
use uuid::Uuid;

/// Processes requests through the MoA pipeline
pub struct RequestProcessor {
    config: Arc<MoaConfig>,
    agent_manager: Arc<AgentManager>,
    strategy_manager: Arc<StrategyManager>,
    resource_manager: Arc<ResourceManager>,
    metrics: RwLock<RequestMetrics>,
}

#[derive(Debug, Default)]
struct RequestMetrics {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    total_latency_ms: u64,
    total_tokens: u64,
}

impl RequestProcessor {
    pub fn new(
        config: Arc<MoaConfig>,
        agent_manager: Arc<AgentManager>,
        strategy_manager: Arc<StrategyManager>,
        resource_manager: Arc<ResourceManager>,
    ) -> MoaResult<Self> {
        Ok(Self {
            config,
            agent_manager,
            strategy_manager,
            resource_manager,
            metrics: RwLock::new(RequestMetrics::default()),
        })
    }

    pub async fn init(&self) -> MoaResult<()> {
        info!("Initializing request processor...");
        Ok(())
    }

    pub async fn shutdown(&self) -> MoaResult<()> {
        info!("Shutting down request processor...");
        Ok(())
    }

    pub async fn health_check(&self) -> MoaResult<HealthStatus> {
        let metrics = self.metrics.read().await;
        if metrics.total_requests > 0 {
            let success_rate = metrics.successful_requests as f64 / metrics.total_requests as f64;
            if success_rate >= 0.95 {
                Ok(HealthStatus::Healthy)
            } else if success_rate >= 0.8 {
                Ok(HealthStatus::Degraded)
            } else {
                Ok(HealthStatus::Unhealthy)
            }
        } else {
            Ok(HealthStatus::Unknown)
        }
    }

    pub async fn detailed_health_check(&self) -> MoaResult<crate::models::ComponentHealthStatus> {
        let status = self.health_check().await?;
        let metrics = self.metrics.read().await;
        
        let message = if metrics.total_requests > 0 {
            let success_rate = metrics.successful_requests as f64 / metrics.total_requests as f64;
            let avg_latency = if metrics.total_requests > 0 {
                metrics.total_latency_ms as f64 / metrics.total_requests as f64
            } else {
                0.0
            };
            Some(format!(
                "Success rate: {:.1}%, Avg latency: {:.1}ms, Total requests: {}",
                success_rate * 100.0,
                avg_latency,
                metrics.total_requests
            ))
        } else {
            Some("No requests processed yet".to_string())
        };

        Ok(crate::models::ComponentHealthStatus {
            status,
            message,
            timestamp: chrono::Utc::now(),
        })
    }

    #[instrument(skip(self, request), fields(request_id = %request.id))]
    pub async fn process_request(&self, request: MoaRequest) -> MoaResult<MoaResponse> {
        let start = Instant::now();
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        drop(metrics); // Release the lock early

        // Validate request
        self.validate_request(&request)?;

        // Get resource permit
        let _permit = self.resource_manager.acquire_permit().await?;

        // Check cache
        if let Some(cached_response) = self.resource_manager.get_cached(&request.id).await {
            debug!("Cache hit for request {}", request.id);
            return Ok(serde_json::from_slice(&cached_response)?);
        }

        // Get strategy
        let strategy = self.strategy_manager.get_strategy(&self.config.strategy).await?;

        // Get agent responses
        let agent_responses = self.collect_agent_responses(&request).await?;

        // Process responses through strategy
        let response = strategy.process_responses(agent_responses, &request).await?;

        // Cache response
        let response_bytes = serde_json::to_vec(&response)?;
        self.resource_manager.cache_response(request.id.clone(), response_bytes).await?;

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.successful_requests += 1;
        metrics.total_latency_ms += start.elapsed().as_millis() as u64;
        metrics.total_tokens += response.metrics.tokens_used as u64;

        Ok(response)
    }

    pub async fn batch_process_requests(&self, requests: Vec<MoaRequest>) -> MoaResult<Vec<MoaResponse>> {
        let mut futures = FuturesUnordered::new();
        for request in requests {
            futures.push(self.process_request(request));
        }

        let mut responses = Vec::new();
        while let Some(result) = futures.next().await {
            responses.push(result?);
        }

        Ok(responses)
    }

    fn validate_request(&self, request: &MoaRequest) -> MoaResult<()> {
        if request.query.is_empty() {
            return Err(MoaError::validation("Query cannot be empty".to_string()));
        }
        Ok(())
    }

    async fn collect_agent_responses(&self, request: &MoaRequest) -> MoaResult<Vec<AgentResponse>> {
        let agents = self.agent_manager.list_agents().await;
        let mut futures = FuturesUnordered::new();

        for (id, _, _) in agents {
            let agent = self.agent_manager.get_agent(&id).await?;
            let request = request.clone();
            futures.push(async move {
                match timeout(
                    std::time::Duration::from_secs(self.config.resources.request_timeout_secs as u64),
                    agent.process_request(&request)
                ).await {
                    Ok(Ok(response)) => Ok(response),
                    Ok(Err(e)) => {
                        warn!("Agent {} failed to process request: {}", id, e);
                        Err(e)
                    },
                    Err(_) => {
                        warn!("Agent {} timed out", id);
                        Err(MoaError::timeout(format!("Agent {} timed out", id)))
                    }
                }
            });
        }

        let mut responses = Vec::new();
        while let Some(result) = futures.next().await {
            match result {
                Ok(response) => responses.push(response),
                Err(e) => warn!("Failed to get agent response: {}", e),
            }
        }

        if responses.is_empty() {
            return Err(MoaError::processing("No agent responses received".to_string()));
        }

        Ok(responses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentRole, AgentType};

    #[tokio::test]
    async fn test_request_processing() {
        // Create test components
        let config = Arc::new(MoaConfig::default());
        let key_manager = Arc::new(crate::security::KeyManager::new("test_keys.json".into()).unwrap());
        let agent_manager = Arc::new(AgentManager::new(&config, key_manager).unwrap());
        let strategy_manager = Arc::new(StrategyManager::new(&config).unwrap());
        let resource_manager = Arc::new(ResourceManager::new(&config).unwrap());

        // Create processor
        let processor = RequestProcessor::new(
            config.clone(),
            agent_manager.clone(),
            strategy_manager.clone(),
            resource_manager.clone(),
        ).unwrap();

        // Create test request
        let request = MoaRequest::new(
            "test query".to_string(),
            None,
        );

        // Process request
        let response = processor.process_request(request).await;
        assert!(response.is_ok());
    }
} 