//! Rule-based agent implementation
//! 
//! This agent uses a rule engine to process requests based on predefined rules.

use crate::{
    agents::{Agent, BaseAgent, AgentMetrics},
    config::{AgentConfig, AgentRole},
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest, ResponseMetrics},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

/// Rule condition type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    /// Match if query contains the text
    Contains(String),
    /// Match if query starts with the text
    StartsWith(String),
    /// Match if query matches the regex pattern
    MatchesRegex(String),
    /// Match if query length is within range
    LengthRange(usize, usize),
    /// Match if all conditions are true
    And(Vec<RuleCondition>),
    /// Match if any condition is true
    Or(Vec<RuleCondition>),
    /// Match if condition is false
    Not(Box<RuleCondition>),
}

/// Rule action type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// Return a static response
    StaticResponse(String),
    /// Transform the query and pass to another agent
    TransformAndDelegate {
        transform: String,
        agent_id: String,
    },
    /// Return an error
    Error(String),
    /// Return a template response with variable substitution
    TemplateResponse {
        template: String,
        variables: HashMap<String, String>,
    },
}

/// Rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub priority: u32,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub enabled: bool,
}

/// Rule engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEngineConfig {
    pub rules: Vec<Rule>,
    pub default_action: RuleAction,
    pub enable_logging: bool,
}

impl Default for RuleEngineConfig {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            default_action: RuleAction::Error("No matching rule found".to_string()),
            enable_logging: true,
        }
    }
}

/// Rule-based agent implementation
#[derive(Debug)]
pub struct RuleBasedAgent {
    base: BaseAgent,
    config: RuleEngineConfig,
    rule_cache: HashMap<String, Rule>,
}

impl RuleBasedAgent {
    pub fn new(
        id: String,
        role: AgentRole,
        config: RuleEngineConfig,
    ) -> Self {
        let mut rule_cache = HashMap::new();
        for rule in &config.rules {
            rule_cache.insert(rule.id.clone(), rule.clone());
        }

        Self {
            base: BaseAgent::new(
                id.clone(),
                "Rule-Based Agent".to_string(),
                "An agent that processes requests using predefined rules".to_string(),
                vec!["rule_processing".to_string(), "pattern_matching".to_string()],
                AgentConfig {
                    name: id.clone(),
                    agent_type: crate::config::AgentType::RuleBased,
                    role: role.clone(),
                    capabilities: vec!["rule_processing".to_string()],
                    config: serde_json::to_value(config.clone()).unwrap_or_default(),
                    max_retries: 0,
                    timeout_secs: 5,
                }
            ),
            config,
            rule_cache,
        }
    }

    /// Evaluate a rule condition with input validation
    fn evaluate_condition(&self, condition: &RuleCondition, query: &str) -> bool {
        // Security: Validate input length to prevent DoS
        if query.len() > 100_000 {
            return false; // Reject extremely long queries
        }
        
        match condition {
            RuleCondition::Contains(text) => {
                // Security: Prevent empty or malicious patterns
                if text.is_empty() || text.len() > 1000 {
                    return false;
                }
                query.contains(text)
            }
            RuleCondition::StartsWith(text) => {
                if text.is_empty() || text.len() > 1000 {
                    return false;
                }
                query.starts_with(text)
            }
            RuleCondition::MatchesRegex(pattern) => {
                // Security: Limit regex pattern length and complexity
                if pattern.is_empty() || pattern.len() > 500 {
                    return false;
                }
                // Simple regex matching (would use regex crate in production with timeout)
                query.contains(pattern)
            }
            RuleCondition::LengthRange(min, max) => {
                let len = query.len();
                len >= *min && len <= *max
            }
            RuleCondition::And(conditions) => {
                conditions.iter().all(|c| self.evaluate_condition(c, query))
            }
            RuleCondition::Or(conditions) => {
                conditions.iter().any(|c| self.evaluate_condition(c, query))
            }
            RuleCondition::Not(condition) => {
                !self.evaluate_condition(condition, query)
            }
        }
    }

    /// Execute a rule action
    async fn execute_action(
        &self,
        action: &RuleAction,
        request: &MoaRequest,
    ) -> MoaResult<String> {
        match action {
            RuleAction::StaticResponse(response) => Ok(response.clone()),
            RuleAction::TransformAndDelegate { transform, agent_id: _ } => {
                // Apply transformation (placeholder)
                let transformed = transform.replace("{query}", &request.query);
                Ok(transformed)
            }
            RuleAction::Error(message) => {
                Err(MoaError::Agent {
                    agent_id: self.get_id().to_string(),
                    message: message.clone(),
                    source: None,
                })
            }
            RuleAction::TemplateResponse { template, variables } => {
                let mut result = template.clone();
                for (key, value) in variables {
                    result = result.replace(&format!("{{{}}}", key), value);
                }
                result = result.replace("{query}", &request.query);
                Ok(result)
            }
        }
    }

    /// Find matching rules for a query
    fn find_matching_rules(&self, query: &str) -> Vec<&Rule> {
        let mut matches: Vec<&Rule> = self.config.rules
            .iter()
            .filter(|rule| rule.enabled && self.evaluate_condition(&rule.condition, query))
            .collect();
        
        // Sort by priority (higher priority first)
        matches.sort_by(|a, b| b.priority.cmp(&a.priority));
        matches
    }
}

#[async_trait]
impl Agent for RuleBasedAgent {
    fn get_id(&self) -> &str {
        self.base.get_id()
    }

    fn get_name(&self) -> &str {
        "Rule-Based Agent"
    }

    fn get_description(&self) -> &str {
        "An agent that processes requests using predefined rules"
    }

    fn get_capabilities(&self) -> &[String] {
        self.base.get_capabilities()
    }

    fn get_config(&self) -> &AgentConfig {
        self.base.get_config()
    }

    async fn process_request(&self, request: &MoaRequest) -> MoaResult<AgentResponse> {
        let start = std::time::Instant::now();
        
        // Find matching rules
        let matching_rules = self.find_matching_rules(&request.query);
        
        if matching_rules.is_empty() {
            // Use default action
            let content = self.execute_action(&self.config.default_action, request).await?;
            let confidence = 0.5; // Lower confidence for default action
            
            let response = AgentResponse {
                id: Uuid::new_v4().to_string(),
                agent_id: self.get_id().to_string(),
                request: request.clone(),
                content,
                confidence,
                timestamp: Utc::now(),
                metrics: ResponseMetrics::default(),
            };
            
            self.base.record_request_outcome(start.elapsed(), confidence, true).await;
            return Ok(response);
        }

        // Use the highest priority matching rule
        let rule = matching_rules[0];
        let content = self.execute_action(&rule.action, request).await?;
        
        // Calculate confidence based on rule priority
        let confidence = (rule.priority as f64 / 100.0).min(1.0);
        
        let response = AgentResponse {
            id: Uuid::new_v4().to_string(),
            agent_id: self.get_id().to_string(),
            request: request.clone(),
            content,
            confidence,
            timestamp: Utc::now(),
            metrics: ResponseMetrics::default(),
        };
        
        self.base.record_request_outcome(start.elapsed(), confidence, true).await;
        Ok(response)
    }

    fn update_config(&mut self, config: AgentConfig) -> MoaResult<()> {
        self.base.update_config(config)
    }

    fn get_metrics(&self) -> AgentMetrics {
        self.base.get_metrics()
    }

    fn reset(&mut self) -> MoaResult<()> {
        self.base.reset()
    }
}

