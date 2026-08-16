use crate::{
    config::AgentRole,
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest, MoaResponse, ResponseMetrics},
    strategies::Strategy,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use chrono::{Utc, DateTime};
use tracing::debug;
use async_trait::async_trait;
use uuid::Uuid;

const MAX_HISTORY_SIZE: usize = 1000;
const PERFORMANCE_DECAY_FACTOR: f32 = 0.95;
const MIN_CONFIDENCE_THRESHOLD: f64 = 0.5;
const SKILL_ADAPTATION_RATE: f32 = 0.1;

/// Role-based agent specialization strategy with adaptive skill profiles
pub struct RolesStrategy {
    /// Role assignments and skill profiles
    assignments: Arc<RwLock<HashMap<String, AgentRoleProfile>>>,
    /// Agent performance history
    performance: Arc<RwLock<HashMap<String, PerformanceHistory>>>,
    /// Minimum confidence threshold
    min_confidence: f64,
    /// Skill adaptation rate
    adaptation_rate: f32,
}

#[derive(Debug, Clone)]
pub struct AgentRoleProfile {
    /// Assigned role
    pub role: AgentRole,
    /// Skill profile
    pub skills: SkillProfile,
    /// Last update timestamp
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PerformanceHistory {
    /// Historical confidence scores with timestamps
    pub scores: Vec<(DateTime<Utc>, f32)>,
    /// Average response time
    pub avg_response_time: Duration,
    /// Success rate
    pub success_rate: f32,
    /// Total requests handled
    pub total_requests: u64,
}

/// Role performance metrics with detailed statistics
#[derive(Debug, Clone)]
pub struct RoleMetrics {
    /// Average confidence score
    pub avg_confidence: f32,
    /// Average quality score
    pub avg_quality_score: f32,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Total number of responses
    pub total_responses: u64,
    /// Success rate
    pub success_rate: f32,
    /// Skill proficiency levels
    pub skill_proficiency: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProfile {
    /// Domain expertise scores
    pub domain_expertise: HashMap<String, f32>,
    /// Task type proficiency
    pub task_proficiency: HashMap<String, f32>,
    /// Specialization areas
    pub specializations: Vec<String>,
    /// Performance metrics
    pub metrics: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average response time in milliseconds
    pub avg_response_time_ms: u64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,
    /// Total requests handled
    pub total_requests: u64,
    /// Error rate
    pub error_rate: f32,
}

impl Default for SkillProfile {
    fn default() -> Self {
        Self {
            domain_expertise: HashMap::new(),
            task_proficiency: HashMap::new(),
            specializations: Vec::new(),
            metrics: PerformanceMetrics {
                avg_response_time_ms: 0,
                success_rate: 0.0,
                total_requests: 0,
                error_rate: 0.0,
            },
        }
    }
}

impl RolesStrategy {
    /// Create a new role strategy with configuration
    pub fn new(min_confidence: f64, adaptation_rate: f32) -> Self {
        Self {
            assignments: Arc::new(RwLock::new(HashMap::new())),
            performance: Arc::new(RwLock::new(HashMap::new())),
            min_confidence,
            adaptation_rate,
        }
    }

    /// Update agent role with skill profile
    pub async fn update_role(
        &self,
        agent_id: &str,
        role: AgentRole,
        skills: Option<SkillProfile>,
    ) -> MoaResult<()> {
        let mut assignments = self.assignments.write().await;
        let profile = AgentRoleProfile {
            role,
            skills: skills.unwrap_or_else(SkillProfile::default),
            last_update: Utc::now(),
        };
        assignments.insert(agent_id.to_string(), profile);
        Ok(())
    }

    /// Get agent role profile
    pub async fn get_role_profile(&self, agent_id: &str) -> Option<AgentRoleProfile> {
        self.assignments.read().await.get(agent_id).cloned()
    }

    /// Update agent performance metrics
    async fn update_performance(&self, agent_id: &str, response: &AgentResponse) {
        let mut performance = self.performance.write().await;
        let history = performance.entry(agent_id.to_string()).or_insert_with(|| PerformanceHistory {
            scores: Vec::new(),
            avg_response_time: Duration::default(),
            success_rate: 0.0,
            total_requests: 0,
        });

        // Update scores
        history.scores.push((Utc::now(), response.confidence as f32));
        if history.scores.len() > MAX_HISTORY_SIZE {
            history.scores.remove(0);
        }

        // Update metrics
        history.total_requests += 1;
        history.avg_response_time = Duration::from_millis(response.metrics.latency_ms);
        history.success_rate = if response.confidence >= self.min_confidence {
            (history.success_rate * PERFORMANCE_DECAY_FACTOR) + (1.0 - PERFORMANCE_DECAY_FACTOR)
        } else {
            history.success_rate * PERFORMANCE_DECAY_FACTOR
        };
    }
}

#[async_trait]
impl Strategy for RolesStrategy {
    async fn process_responses(
        &self,
        responses: Vec<AgentResponse>,
        _request: &MoaRequest,
    ) -> MoaResult<MoaResponse> {
        if responses.is_empty() {
            return Err(MoaError::Strategy {
                message: "No responses to process".to_string(),
                source: None
            });
        }

        // Update performance metrics for all agents
        for response in &responses {
            self.update_performance(&response.agent_id, response).await;
        }

        // Find best response based on confidence and role performance
        let best_response = responses.into_iter()
            .max_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap())
            .ok_or_else(|| MoaError::Strategy {
                message: "No response met confidence threshold".to_string(),
                source: None
            })?;

        let metrics = best_response.metrics.clone();
        Ok(MoaResponse {
            id: Uuid::new_v4(),
            content: best_response.content.clone(),
            confidence: best_response.confidence as f32,
            agent_responses: vec![best_response],
            timestamp: Utc::now(),
            metrics: ResponseMetrics {
                latency_ms: metrics.latency_ms,
                tokens_used: metrics.tokens_used,
                prompt_tokens: metrics.prompt_tokens,
                completion_tokens: metrics.completion_tokens,
            },
        })
    }

    async fn warmup(&self) -> MoaResult<()> {
        debug!("Warming up roles strategy");
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        debug!("Shutting down roles strategy");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentMetrics;
    use std::time::Duration;

    #[tokio::test]
    async fn test_role_strategy() {
        let strategy = RolesStrategy::new(0.5, 0.1);
        
        // Test role assignment
        let agent_id = "test_agent";
        let mut skill_profile = SkillProfile::default();
        skill_profile.domain_expertise.insert("coding".to_string(), 0.8);
        skill_profile.domain_expertise.insert("analysis".to_string(), 0.7);
        
        strategy.update_role(
            agent_id,
            AgentRole::Primary,
            Some(skill_profile),
        ).await.unwrap();
        
        let profile = strategy.get_role_profile(agent_id).await.unwrap();
        assert!(matches!(profile.role, AgentRole::Primary));
        
        // Test performance tracking
        let request = MoaRequest::new("test".to_string(), None);
        let response = AgentResponse {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            request: request.clone(),
            content: "Test content".to_string(),
            confidence: 0.9,
            timestamp: Utc::now(),
            metrics: ResponseMetrics {
                latency_ms: 100,
                tokens_used: 50,
                prompt_tokens: 20,
                completion_tokens: 30,
            },
        };
        
        strategy.update_performance(agent_id, &response).await;
        
        let performance = strategy.performance.read().await;
        let history = performance.get(agent_id).unwrap();
        assert_eq!(history.total_requests, 1);
        assert!(history.success_rate > 0.0);
    }

    #[tokio::test]
    async fn test_skill_profile() {
        let mut profile = SkillProfile::default();
        
        // Test domain expertise updates
        profile.domain_expertise.insert("coding".to_string(), 0.8);
        profile.domain_expertise.insert("coding".to_string(), 0.9);
        
        assert!(profile.domain_expertise.get("coding").unwrap() > &0.8);
        assert_eq!(profile.domain_expertise.get("unknown"), None);

        // Test specializations
        profile.specializations.push("coding".to_string());
        assert_eq!(profile.specializations.len(), 1);
    }
} 