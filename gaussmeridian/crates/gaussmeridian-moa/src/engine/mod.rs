pub mod agent_manager;
pub mod request_processor;
pub mod resource_manager;
pub mod strategy_manager;

use crate::{
    config::MoaConfig,
    error::MoaResult,
    models::{MoaRequest, MoaResponse, HealthStatus, DetailedHealthStatus},
    providers::{ChatProvider, HttpChatProvider},
};
use agent_manager::AgentManager;
use request_processor::RequestProcessor;
use resource_manager::ResourceManager;
use strategy_manager::StrategyManager;
use std::{path::PathBuf, sync::Arc};
use tracing::info;

/// Main MoA engine that orchestrates all components
pub struct MoaEngine {
    config: Arc<MoaConfig>,
    agent_manager: Arc<AgentManager>,
    request_processor: Arc<RequestProcessor>,
    resource_manager: Arc<ResourceManager>,
    strategy_manager: Arc<StrategyManager>,
}

impl MoaEngine {
    /// Create a new MoA engine from a config file (standalone/debug mode). Uses the built-in
    /// OpenAI-compatible provider from the environment. The gateway uses [`MoaEngine::from_parts`].
    pub async fn new(config_path: impl Into<PathBuf>) -> MoaResult<Self> {
        let config = crate::config::load_config(config_path.into())?;
        Self::from_parts(config, Arc::new(HttpChatProvider::from_env())).await
    }

    /// Build the engine **in-process** from an explicit config plus an injected chat provider —
    /// no config file, no key file, no second process. The GaussMeridian gateway constructs the
    /// engine this way, supplying its shared `gaussmeridian-providers` stack (Seam 3).
    pub async fn from_parts(
        config: MoaConfig,
        provider: Arc<dyn ChatProvider>,
    ) -> MoaResult<Self> {
        let config = Arc::new(config);

        let resource_manager = Arc::new(ResourceManager::new(&config)?);
        let agent_manager = Arc::new(AgentManager::new(&config, provider)?);
        let strategy_manager = Arc::new(StrategyManager::new(&config)?);
        let request_processor = Arc::new(RequestProcessor::new(
            config.clone(),
            agent_manager.clone(),
            strategy_manager.clone(),
            resource_manager.clone(),
        )?);

        let engine = Self {
            config,
            agent_manager,
            request_processor,
            resource_manager,
            strategy_manager,
        };

        engine.init().await?;
        Ok(engine)
    }

    /// Initialize all components
    pub async fn init(&self) -> MoaResult<()> {
        info!("Initializing MoA engine components...");
        self.resource_manager.init().await?;
        self.agent_manager.init().await?;
        self.strategy_manager.init().await?;
        self.request_processor.init().await?;
        info!("MoA engine initialization complete");
        Ok(())
    }

    /// Warm up components for optimal performance
    pub async fn warmup(&self) -> MoaResult<()> {
        info!("Warming up MoA engine components...");
        self.agent_manager.warmup().await?;
        self.strategy_manager.warmup().await?;
        info!("MoA engine warmup complete");
        Ok(())
    }

    /// Gracefully shutdown all components
    pub async fn shutdown(&self) -> MoaResult<()> {
        info!("Shutting down MoA engine components...");
        self.request_processor.shutdown().await?;
        self.agent_manager.shutdown().await?;
        self.strategy_manager.shutdown().await?;
        self.resource_manager.shutdown().await?;
        info!("MoA engine shutdown complete");
        Ok(())
    }

    /// Quick health check of all components
    pub async fn health_check(&self) -> MoaResult<HealthStatus> {
        let resource_health = self.resource_manager.health_check().await?;
        let agent_health = self.agent_manager.health_check().await?;
        let strategy_health = self.strategy_manager.health_check().await?;
        let processor_health = self.request_processor.health_check().await?;

        if resource_health.is_healthy() && 
           agent_health.is_healthy() && 
           strategy_health.is_healthy() && 
           processor_health.is_healthy() {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Unhealthy)
        }
    }

    /// Detailed health check with component-specific information
    pub async fn deep_health_check(&self) -> MoaResult<DetailedHealthStatus> {
        Ok(DetailedHealthStatus {
            resources: self.resource_manager.detailed_health_check().await?,
            agents: self.agent_manager.detailed_health_check().await?,
            strategies: self.strategy_manager.detailed_health_check().await?,
            processor: self.request_processor.detailed_health_check().await?,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Process a query through the MoA pipeline
    pub async fn process_query(
        &self,
        query: &str,
        context: Option<&str>
    ) -> MoaResult<MoaResponse> {
        let request = MoaRequest::new(query.to_string(), context.map(String::from));
        self.request_processor.process_request(request).await
    }

    /// Process multiple queries in batch
    pub async fn batch_process_queries(
        &self,
        queries: Vec<(&str, Option<&str>)>
    ) -> MoaResult<Vec<MoaResponse>> {
        let requests: Vec<MoaRequest> = queries
            .into_iter()
            .map(|(query, context)| MoaRequest::new(query.to_string(), context.map(String::from)))
            .collect();
        self.request_processor.batch_process_requests(requests).await
    }

    /// Get agent manager reference
    pub fn agent_manager(&self) -> &Arc<AgentManager> {
        &self.agent_manager
    }

    /// Get request processor reference
    pub fn request_processor(&self) -> &Arc<RequestProcessor> {
        &self.request_processor
    }

    /// Get strategy manager reference
    pub fn strategy_manager(&self) -> &Arc<StrategyManager> {
        &self.strategy_manager
    }

    /// Get resource manager reference
    pub fn resource_manager(&self) -> &Arc<ResourceManager> {
        &self.resource_manager
    }
}

impl Drop for MoaEngine {
    fn drop(&mut self) {
        info!("MoA engine is being dropped");
    }
} 