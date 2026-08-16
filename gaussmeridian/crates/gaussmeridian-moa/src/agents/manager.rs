use crate::{
    config::{AgentConfig, AgentRole, ModelType},
    error::{MoaError, MoaResult},
    agents::{Agent, AgentMetrics, LlmAgent},
    security::KeyManager,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock as TokioRwLock;
use tracing::{debug, error, info, warn};

/// Manager for handling multiple agents
pub struct AgentManager {
    /// Active agents
    agents: DashMap<String, Box<dyn Agent>>,
    /// Agent performance history
    history: Arc<TokioRwLock<DashMap<String, Vec<f32>>>>,
    /// Key manager for agent credentials
    key_manager: Arc<KeyManager>,
}

impl AgentManager {
    /// Create a new agent manager
    pub fn new(key_manager: Arc<KeyManager>) -> Self {
        Self {
            agents: DashMap::new(),
            history: Arc::new(TokioRwLock::new(DashMap::new())),
            key_manager,
        }
    }
    
    /// Add a new agent
    pub fn add_agent(&self, agent: Box<dyn Agent>) -> MoaResult<()> {
        let id = agent.id().to_string();
        if self.agents.contains_key(&id) {
            return Err(MoaError::Configuration(format!("Agent '{}' already exists", id)));
        }

        self.agents.insert(id.clone(), agent);
        self.history.blocking_write().insert(id.clone(), Vec::new());
        
        info!("Added agent: {}", id);
        Ok(())
    }
    
    /// Remove an agent
    pub fn remove_agent(&self, id: &str) -> MoaResult<()> {
        self.agents.remove(id)
            .ok_or_else(|| MoaError::Configuration(format!("Agent '{}' not found", id)))?;
            
        self.history.blocking_write().remove(id);
        
        info!("Removed agent: {}", id);
        Ok(())
    }
    
    /// Get an agent by ID
    pub fn get_agent(&self, id: &str) -> Option<Box<dyn Agent>> {
        self.agents.get(id).map(|a| a.clone())
    }
    
    /// List all agents with their roles and performance
    pub fn list_agents(&self) -> Vec<(String, AgentRole, f32)> {
        self.agents.iter()
            .map(|entry| {
                let id = entry.key().clone();
                let role = entry.value().role();
                let performance = self.get_agent_performance(&id);
                (id, role, performance)
            })
            .collect()
    }
    
    /// Get agent metrics
    pub fn get_metrics(&self, id: &str) -> Option<AgentMetrics> {
        self.agents.get(id).map(|agent| agent.metrics())
    }
    
    /// Update agent performance history
    pub async fn update_performance(&self, id: &str, confidence: f32) {
        let mut history = self.history.write().await;
        if let Some(scores) = history.get_mut(id) {
            scores.push(confidence);
            if scores.len() > 100 {
                scores.remove(0);
            }
        }
    }
    
    /// Get agent's average performance
    pub fn get_agent_performance(&self, id: &str) -> f32 {
        self.history.blocking_read()
            .get(id)
            .map(|scores| {
                if scores.is_empty() {
                    0.5
                } else {
                    scores.iter().sum::<f32>() / scores.len() as f32
                }
            })
            .unwrap_or(0.5)
    }
    
    /// Get best performing agent
    pub fn get_best_agent(&self) -> Option<String> {
        self.agents.iter()
            .map(|entry| {
                let id = entry.key().clone();
                let performance = self.get_agent_performance(&id);
                (id, performance)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(id, _)| id)
    }
    
    /// Get agents by role
    pub fn get_agents_by_role(&self, role: &AgentRole) -> Vec<Box<dyn Agent>> {
        self.agents.iter()
            .filter(|entry| entry.value().role() == *role)
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    /// Configure agents from configuration
    pub fn configure_agents(&self, configs: &[AgentConfig]) -> MoaResult<()> {
        for config in configs {
            let agent = crate::agents::create_agent(config, self.key_manager.clone())?;
            self.add_agent(agent)?;
        }
        Ok(())
    }
    
    /// Get number of active agents
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
    
    /// Check if manager has any agents
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
    
    /// Clear all agents
    pub fn clear(&self) {
        self.agents.clear();
        self.history.blocking_write().clear();
        info!("Cleared all agents");
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
} 