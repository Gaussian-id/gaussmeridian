use crate::{
    config::{MoaConfig, MoaStrategy},
    error::{MoaError, MoaResult},
    models::HealthStatus,
    strategies::{Strategy, create_strategy},
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Manages strategy lifecycle and operations
pub struct StrategyManager {
    config: Arc<MoaConfig>,
    strategies: RwLock<HashMap<String, Box<dyn Strategy>>>,
    active_strategy: RwLock<String>,
}

impl StrategyManager {
    pub fn new(config: &Arc<MoaConfig>) -> MoaResult<Self> {
        Ok(Self {
            config: Arc::clone(config),
            strategies: RwLock::new(HashMap::new()),
            active_strategy: RwLock::new(String::new()),
        })
    }

    pub async fn init(&self) -> MoaResult<()> {
        info!("Initializing strategy manager...");
        let mut strategies = self.strategies.write().await;
        let mut active = self.active_strategy.write().await;

        // Initialize default strategies
        strategies.insert("standard".to_string(), create_strategy("standard")?);
        strategies.insert("sparse".to_string(), create_strategy("sparse")?);
        strategies.insert("self_moa".to_string(), create_strategy("self_moa")?);
        strategies.insert("collaborative".to_string(), create_strategy("collaborative")?);
        strategies.insert("adaptive".to_string(), create_strategy("adaptive")?);

        // Set active strategy based on config
        let strategy_name = match &self.config.strategy {
            MoaStrategy::Standard(_) => "standard",
            MoaStrategy::Sparse(_) => "sparse",
            MoaStrategy::SelfMoa(_) => "self_moa",
            MoaStrategy::Collaborative(_) => "collaborative",
            MoaStrategy::Adaptive(_) => "adaptive",
        };
        *active = strategy_name.to_string();

        info!("Strategy manager initialization complete");
        Ok(())
    }

    pub async fn warmup(&self) -> MoaResult<()> {
        info!("Warming up strategies...");
        let strategies = self.strategies.read().await;
        for (name, strategy) in strategies.iter() {
            if let Err(e) = strategy.warmup().await {
                warn!("Strategy {} warmup failed: {}", name, e);
            }
        }
        Ok(())
    }

    pub async fn shutdown(&self) -> MoaResult<()> {
        info!("Shutting down strategies...");
        let strategies = self.strategies.read().await;
        for (name, strategy) in strategies.iter() {
            if let Err(e) = strategy.shutdown().await {
                error!("Failed to shutdown strategy {}: {}", name, e);
            }
        }
        Ok(())
    }

    pub async fn health_check(&self) -> MoaResult<HealthStatus> {
        let strategies = self.strategies.read().await;
        let active = self.active_strategy.read().await;
        
        if let Some(strategy) = strategies.get(&*active) {
            strategy.health_check().await
        } else {
            Ok(HealthStatus::Unhealthy)
        }
    }

    pub async fn detailed_health_check(&self) -> MoaResult<crate::models::ComponentHealthStatus> {
        let status = self.health_check().await?;
        let strategies = self.strategies.read().await;
        let active = self.active_strategy.read().await;
        
        let message = if strategies.is_empty() {
            Some("No strategies configured".to_string())
        } else if !strategies.contains_key(&*active) {
            Some(format!("Active strategy '{}' not found", active))
        } else {
            None
        };

        Ok(crate::models::ComponentHealthStatus {
            status,
            message,
            timestamp: chrono::Utc::now(),
        })
    }

    pub async fn get_strategy(&self, strategy_type: &MoaStrategy) -> MoaResult<Box<dyn Strategy>> {
        let strategies = self.strategies.read().await;
        let strategy_name = match strategy_type {
            MoaStrategy::Standard(_) => "standard",
            MoaStrategy::Sparse(_) => "sparse",
            MoaStrategy::SelfMoa(_) => "self_moa",
            MoaStrategy::Collaborative(_) => "collaborative",
            MoaStrategy::Adaptive(_) => "adaptive",
        };

        strategies.get(strategy_name)
            .cloned()
            .ok_or_else(|| MoaError::strategy(format!("Strategy {} not found", strategy_name)))
    }

    pub async fn add_strategy(&self, name: &str, strategy: Box<dyn Strategy>) -> MoaResult<()> {
        let mut strategies = self.strategies.write().await;
        if strategies.contains_key(name) {
            return Err(MoaError::strategy(format!("Strategy {} already exists", name)));
        }
        strategies.insert(name.to_string(), strategy);
        Ok(())
    }

    pub async fn remove_strategy(&self, name: &str) -> MoaResult<()> {
        let mut strategies = self.strategies.write().await;
        let active = self.active_strategy.read().await;
        
        if *active == name {
            return Err(MoaError::strategy(format!("Cannot remove active strategy {}", name)));
        }
        
        strategies.remove(name)
            .ok_or_else(|| MoaError::strategy(format!("Strategy {} not found", name)))?;
        Ok(())
    }

    pub async fn set_active_strategy(&self, name: &str) -> MoaResult<()> {
        let strategies = self.strategies.read().await;
        if !strategies.contains_key(name) {
            return Err(MoaError::strategy(format!("Strategy {} not found", name)));
        }
        
        let mut active = self.active_strategy.write().await;
        *active = name.to_string();
        Ok(())
    }

    pub async fn get_active_strategy(&self) -> MoaResult<String> {
        Ok(self.active_strategy.read().await.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_strategy_management() {
        // Create test config
        let config = Arc::new(MoaConfig::default());
        
        // Create strategy manager
        let manager = StrategyManager::new(&config).unwrap();
        
        // Test initialization
        manager.init().await.unwrap();
        
        // Test strategy retrieval
        let strategy = manager.get_strategy(&MoaStrategy::Standard(Default::default())).await;
        assert!(strategy.is_ok());
        
        // Test health check
        let health = manager.health_check().await.unwrap();
        assert!(matches!(health, HealthStatus::Healthy));
        
        // Test shutdown
        manager.shutdown().await.unwrap();
    }
} 