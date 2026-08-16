//! Strategy persistence implementation
//! 
//! This module provides functionality to persist and recover strategy state.

use crate::{
    error::MoaResult,
    models::AgentResponse,
    strategies::Strategy,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Strategy state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyState {
    pub strategy_id: String,
    pub strategy_type: String,
    pub state_data: serde_json::Value,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Performance history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub performance_score: f32,
    pub response_time_ms: u64,
    pub confidence: f64,
    pub success: bool,
}

/// Adaptation history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationHistoryEntry {
    pub strategy_id: String,
    pub timestamp: DateTime<Utc>,
    pub change_type: String,
    pub change_description: String,
    pub before_state: serde_json::Value,
    pub after_state: serde_json::Value,
}

/// Strategy history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyHistory {
    pub strategy_id: String,
    pub performance_history: Vec<PerformanceHistoryEntry>,
    pub adaptation_history: Vec<AdaptationHistoryEntry>,
    pub last_updated: DateTime<Utc>,
}

/// Strategy persistence trait
#[async_trait]
pub trait StrategyPersistence: Send + Sync {
    /// Save strategy state
    async fn save_state(&self, state: StrategyState) -> MoaResult<()>;
    
    /// Load strategy state
    async fn load_state(&self, strategy_id: &str) -> MoaResult<Option<StrategyState>>;
    
    /// Save performance history entry
    async fn save_performance_entry(&self, entry: PerformanceHistoryEntry) -> MoaResult<()>;
    
    /// Get performance history
    async fn get_performance_history(&self, strategy_id: &str, limit: Option<usize>) -> MoaResult<Vec<PerformanceHistoryEntry>>;
    
    /// Save adaptation history entry
    async fn save_adaptation_entry(&self, entry: AdaptationHistoryEntry) -> MoaResult<()>;
    
    /// Get adaptation history
    async fn get_adaptation_history(&self, strategy_id: &str, limit: Option<usize>) -> MoaResult<Vec<AdaptationHistoryEntry>>;
    
    /// Get full strategy history
    async fn get_strategy_history(&self, strategy_id: &str) -> MoaResult<StrategyHistory>;
}

/// In-memory strategy persistence implementation
pub struct InMemoryStrategyPersistence {
    states: Arc<RwLock<HashMap<String, StrategyState>>>,
    performance_history: Arc<RwLock<HashMap<String, Vec<PerformanceHistoryEntry>>>>,
    adaptation_history: Arc<RwLock<HashMap<String, Vec<AdaptationHistoryEntry>>>>,
}

impl InMemoryStrategyPersistence {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            performance_history: Arc::new(RwLock::new(HashMap::new())),
            adaptation_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryStrategyPersistence {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StrategyPersistence for InMemoryStrategyPersistence {
    async fn save_state(&self, state: StrategyState) -> MoaResult<()> {
        let mut states = self.states.write().await;
        states.insert(state.strategy_id.clone(), state);
        Ok(())
    }

    async fn load_state(&self, strategy_id: &str) -> MoaResult<Option<StrategyState>> {
        let states = self.states.read().await;
        Ok(states.get(strategy_id).cloned())
    }

    async fn save_performance_entry(&self, entry: PerformanceHistoryEntry) -> MoaResult<()> {
        let mut history = self.performance_history.write().await;
        let strategy_id = entry.agent_id.clone(); // Using agent_id as strategy_id for now
        history.entry(strategy_id).or_insert_with(Vec::new).push(entry);
        Ok(())
    }

    async fn get_performance_history(&self, strategy_id: &str, limit: Option<usize>) -> MoaResult<Vec<PerformanceHistoryEntry>> {
        let history = self.performance_history.read().await;
        let entries = history.get(strategy_id).cloned().unwrap_or_default();
        Ok(if let Some(limit) = limit {
            entries.into_iter().rev().take(limit).rev().collect()
        } else {
            entries
        })
    }

    async fn save_adaptation_entry(&self, entry: AdaptationHistoryEntry) -> MoaResult<()> {
        let mut history = self.adaptation_history.write().await;
        history.entry(entry.strategy_id.clone()).or_insert_with(Vec::new).push(entry);
        Ok(())
    }

    async fn get_adaptation_history(&self, strategy_id: &str, limit: Option<usize>) -> MoaResult<Vec<AdaptationHistoryEntry>> {
        let history = self.adaptation_history.write().await;
        let entries = history.get(strategy_id).cloned().unwrap_or_default();
        Ok(if let Some(limit) = limit {
            entries.into_iter().rev().take(limit).rev().collect()
        } else {
            entries
        })
    }

    async fn get_strategy_history(&self, strategy_id: &str) -> MoaResult<StrategyHistory> {
        let perf_history = self.get_performance_history(strategy_id, None).await?;
        let adapt_history = self.get_adaptation_history(strategy_id, None).await?;
        
        let perf_timestamps = perf_history.iter().map(|e| e.timestamp);
        let adapt_timestamps = adapt_history.iter().map(|e| e.timestamp);
        let last_updated = perf_timestamps
            .chain(adapt_timestamps)
            .max()
            .unwrap_or_else(Utc::now);
        
        Ok(StrategyHistory {
            strategy_id: strategy_id.to_string(),
            performance_history: perf_history,
            adaptation_history: adapt_history,
            last_updated,
        })
    }
}

/// Strategy with persistence support
pub struct PersistentStrategy {
    strategy: Box<dyn Strategy>,
    persistence: Arc<dyn StrategyPersistence>,
    strategy_id: String,
}

impl PersistentStrategy {
    pub fn new(
        strategy: Box<dyn Strategy>,
        persistence: Arc<dyn StrategyPersistence>,
        strategy_id: String,
    ) -> Self {
        Self {
            strategy,
            persistence,
            strategy_id,
        }
    }

    /// Save current strategy state
    pub async fn save_state(&self, state_data: serde_json::Value) -> MoaResult<()> {
        let state = StrategyState {
            strategy_id: self.strategy_id.clone(),
            strategy_type: "unknown".to_string(), // Would be determined from strategy type
            state_data,
            version: "1.0".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.persistence.save_state(state).await
    }

    /// Load and restore strategy state
    pub async fn load_state(&self) -> MoaResult<Option<serde_json::Value>> {
        Ok(self.persistence.load_state(&self.strategy_id).await?.map(|s| s.state_data))
    }

    /// Record performance entry
    pub async fn record_performance(
        &self,
        agent_id: String,
        performance_score: f32,
        response_time_ms: u64,
        confidence: f64,
        success: bool,
    ) -> MoaResult<()> {
        let entry = PerformanceHistoryEntry {
            timestamp: Utc::now(),
            agent_id,
            performance_score,
            response_time_ms,
            confidence,
            success,
        };
        self.persistence.save_performance_entry(entry).await
    }

    /// Record adaptation entry
    pub async fn record_adaptation(
        &self,
        change_type: String,
        change_description: String,
        before_state: serde_json::Value,
        after_state: serde_json::Value,
    ) -> MoaResult<()> {
        let entry = AdaptationHistoryEntry {
            strategy_id: self.strategy_id.clone(),
            timestamp: Utc::now(),
            change_type,
            change_description,
            before_state,
            after_state,
        };
        self.persistence.save_adaptation_entry(entry).await
    }
}

#[async_trait]
impl Strategy for PersistentStrategy {
    async fn process_responses(
        &self,
        responses: Vec<AgentResponse>,
        request: &crate::models::MoaRequest,
    ) -> MoaResult<crate::models::MoaResponse> {
        let result = self.strategy.process_responses(responses, request).await;
        
        // Record performance if successful
        if let Ok(ref response) = result {
            // Record performance for each agent response
            for agent_response in &response.agent_responses {
                let _ = self.record_performance(
                    agent_response.agent_id.clone(),
                    1.0, // Performance score
                    response.metrics.latency_ms,
                    agent_response.confidence,
                    true,
                ).await;
            }
        }
        
        result
    }

    async fn warmup(&self) -> MoaResult<()> {
        // Try to restore state on warmup
        if let Some(state) = self.load_state().await? {
            // State restoration would happen here
            // For now, just warmup the underlying strategy
        }
        self.strategy.warmup().await
    }

    async fn shutdown(&self) -> MoaResult<()> {
        // Save state on shutdown
        // This would serialize the current strategy state
        let _ = self.save_state(serde_json::json!({})).await;
        self.strategy.shutdown().await
    }

    async fn health_check(&self) -> MoaResult<bool> {
        self.strategy.health_check().await
    }
}

