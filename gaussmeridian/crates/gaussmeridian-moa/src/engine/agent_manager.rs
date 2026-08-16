use crate::{
    agents::{Agent, AgentMetrics, Capability},
    config::{AgentConfig, MoaConfig},
    error::{MoaError, MoaResult},
    models::{AgentHealth, DetailedAgentHealth},
    providers::ChatProvider,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use async_trait::async_trait;

/// Manages agent lifecycle and operations
pub struct AgentManager {
    agents: RwLock<HashMap<String, Arc<dyn Agent>>>,
    config: Arc<MoaConfig>,
    provider: Arc<dyn ChatProvider>,
    health_status: RwLock<HashMap<String, AgentHealth>>,
}

impl AgentManager {
    pub fn new(config: &Arc<MoaConfig>, provider: Arc<dyn ChatProvider>) -> MoaResult<Self> {
        Ok(Self {
            agents: RwLock::new(HashMap::new()),
            config: Arc::clone(config),
            provider,
            health_status: RwLock::new(HashMap::new()),
        })
    }

    pub async fn init(&self) -> MoaResult<()> {
        info!("Initializing agent manager...");
        let mut agents = self.agents.write().await;
        let mut health_status = self.health_status.write().await;

        for agent_config in &self.config.agents {
            let agent = crate::agents::create_agent(agent_config, self.provider.clone())?;
            let agent_id = agent.get_id().to_string();
            
            // Initialize agent
            if let Err(e) = agent.init().await {
                error!("Failed to initialize agent {}: {}", agent_id, e);
                health_status.insert(agent_id.clone(), AgentHealth::Unhealthy);
                continue;
            }

            agents.insert(agent_id.clone(), Arc::from(agent));
            health_status.insert(agent_id, AgentHealth::Healthy);
        }

        info!("Agent manager initialization complete");
        Ok(())
    }

    pub async fn warmup(&self) -> MoaResult<()> {
        info!("Warming up agents...");
        let agents = self.agents.read().await;
        let mut health_status = self.health_status.write().await;

        for (id, agent) in agents.iter() {
            if let Err(e) = agent.warmup().await {
                warn!("Agent {} warmup failed: {}", id, e);
                health_status.insert(id.clone(), AgentHealth::Degraded);
            }
        }

        info!("Agent warmup complete");
        Ok(())
    }

    pub async fn shutdown(&self) -> MoaResult<()> {
        info!("Shutting down agents...");
        let agents = self.agents.read().await;
        
        for (id, agent) in agents.iter() {
            if let Err(e) = agent.shutdown().await {
                error!("Failed to shutdown agent {}: {}", id, e);
            }
        }

        info!("Agent shutdown complete");
        Ok(())
    }

    pub async fn add_agent(&self, agent: Box<dyn Agent>) -> MoaResult<()> {
        let agent_id = agent.get_id().to_string();
        let mut agents = self.agents.write().await;
        let mut health_status = self.health_status.write().await;

        if agents.contains_key(&agent_id) {
            return Err(MoaError::agent(
                "Agent already exists".to_string(),
                agent_id,
                None::<Box<dyn std::error::Error + Send + Sync>>,
            ));
        }

        // Initialize the new agent
        agent.init().await?;
        
        agents.insert(agent_id.clone(), Arc::from(agent));
        health_status.insert(agent_id, AgentHealth::Healthy);
        
        Ok(())
    }

    pub async fn remove_agent(&self, agent_id: &str) -> MoaResult<()> {
        let mut agents = self.agents.write().await;
        let mut health_status = self.health_status.write().await;

        if let Some(agent) = agents.remove(agent_id) {
            agent.shutdown().await?;
            health_status.remove(agent_id);
            Ok(())
        } else {
            Err(MoaError::not_found(
                format!("Agent '{}' not found", agent_id),
                None::<String>,
            ))
        }
    }

    pub async fn get_agent(&self, agent_id: &str) -> MoaResult<Arc<dyn Agent>> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned().ok_or_else(|| {
            MoaError::not_found(
                format!("Agent '{}' not found", agent_id),
                None::<String>,
            )
        })
    }

    pub async fn list_agents(&self) -> Vec<(String, AgentConfig, AgentMetrics)> {
        let agents = self.agents.read().await;
        agents
            .iter()
            .map(|(id, agent)| {
                (
                    id.clone(),
                    agent.get_config().clone(),
                    agent.get_metrics(),
                )
            })
            .collect()
    }

    pub async fn health_check(&self) -> MoaResult<AgentHealth> {
        let health_status = self.health_status.read().await;
        let unhealthy_count = health_status
            .values()
            .filter(|&status| *status == AgentHealth::Unhealthy)
            .count();

        if unhealthy_count == 0 {
            Ok(AgentHealth::Healthy)
        } else if unhealthy_count < health_status.len() {
            Ok(AgentHealth::Degraded)
        } else {
            Ok(AgentHealth::Unhealthy)
        }
    }

    pub async fn detailed_health_check(&self) -> MoaResult<DetailedAgentHealth> {
        let agents = self.agents.read().await;
        let health_status = self.health_status.read().await;
        let mut agent_statuses = HashMap::new();

        for (id, agent) in agents.iter() {
            let metrics = agent.get_metrics();
            let health = health_status.get(id).cloned().unwrap_or(AgentHealth::Unknown);
            agent_statuses.insert(id.clone(), (health, metrics));
        }

        Ok(DetailedAgentHealth {
            agent_statuses,
            total_agents: agents.len(),
            timestamp: chrono::Utc::now(),
        })
    }

    pub async fn discover_capabilities(&self, agent_id: &str) -> MoaResult<Vec<Capability>> {
        let agent = self.get_agent(agent_id).await?;
        agent.discover_capabilities().await
    }

    pub async fn negotiate_capabilities(
        &self,
        agent_id: &str,
        required: &[Capability],
    ) -> MoaResult<Vec<Capability>> {
        let agent = self.get_agent(agent_id).await?;
        agent.negotiate_capabilities(required).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic in-test provider — no network. Proves the manager builds agents against an
    /// injected `ChatProvider` (Seam 2) rather than a hardcoded HTTP client.
    #[derive(Debug)]
    struct MockProvider;

    #[async_trait]
    impl ChatProvider for MockProvider {
        async fn complete(
            &self,
            model: &str,
            _prompt: &str,
            _temperature: f32,
            _max_tokens: usize,
        ) -> MoaResult<String> {
            Ok(format!("mock<{model}>"))
        }
    }

    #[tokio::test]
    async fn manager_builds_from_a_mock_provider() {
        let config = Arc::new(MoaConfig::default());
        let manager = AgentManager::new(&config, Arc::new(MockProvider)).unwrap();
        manager.init().await.unwrap();
        // A default config carries no agents; init succeeds and health is Healthy (no unhealthy).
        let health = manager.health_check().await.unwrap();
        assert!(matches!(health, AgentHealth::Healthy));
        manager.shutdown().await.unwrap();
    }
}
