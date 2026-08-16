use async_trait::async_trait;
use std::{
    fmt::Debug,
    collections::HashMap,
    sync::Arc,
};

pub mod base;
pub mod llm_agent;
pub mod self_moa;
pub mod sparse;
pub mod rule_based;
pub mod retrieval;

pub use base::BaseAgent;
pub use llm_agent::{LlmAgent, LlmAgentConfig, LlmProvider};
pub use self_moa::SelfMoaAgent;
pub use sparse::SparseMoaAgent;

use crate::{
    config::{AgentConfig, AgentRole, AgentType},
    error::{MoaError, MoaResult},
    providers::ChatProvider,
    models::{AgentResponse, MoaRequest},
};

pub mod metrics;
pub use metrics::AgentMetrics;

/// Core trait that all agents must implement
#[async_trait]
pub trait Agent: Send + Sync + Debug {
    /// Get the agent's unique identifier
    fn get_id(&self) -> &str;
    
    /// Get the agent's display name
    fn get_name(&self) -> &str;
    
    /// Get the agent's description
    fn get_description(&self) -> &str;
    
    /// Get the agent's capabilities
    fn get_capabilities(&self) -> &[String];
    
    /// Get the agent's configuration
    fn get_config(&self) -> &AgentConfig;
    
    /// Process a request and generate a response
    async fn process_request(&self, request: &MoaRequest) -> MoaResult<AgentResponse>;
    
    /// Update the agent's configuration
    fn update_config(&mut self, config: AgentConfig) -> MoaResult<()>;
    
    /// Get the agent's performance metrics
    fn get_metrics(&self) -> AgentMetrics;
    
    /// Reset the agent's state
    fn reset(&mut self) -> MoaResult<()>;

    /// Generate a response for a request
    async fn generate_response(&self, request: &MoaRequest) -> MoaResult<AgentResponse> {
        self.process_request(request).await
    }
}

/// Create an agent from configuration. `provider` is the shared chat provider the LLM agents
/// call (the gateway injects its `gaussmeridian-providers`-backed adapter; standalone uses the
/// built-in `HttpChatProvider`). Non-LLM agents ignore it.
pub fn create_agent(
    config: &AgentConfig,
    provider: Arc<dyn ChatProvider>,
) -> MoaResult<Box<dyn Agent>> {
    match &config.agent_type {
        AgentType::LLM => {
            let llm_config: LlmAgentConfig = serde_json::from_value(config.config.clone())
                .map_err(|e| MoaError::Config {
                    message: format!("Invalid LLM agent config: {}", e),
                    source: Some(Box::new(e)),
                })?;
            Ok(Box::new(LlmAgent::new(
                config.name.clone(),
                config.role.clone(),
                llm_config,
                provider,
            )))
        }
        AgentType::RuleBased => {
            use rule_based::{RuleBasedAgent, RuleEngineConfig};
            let rule_config: RuleEngineConfig = serde_json::from_value(config.config.clone())
                .map_err(|e| MoaError::Config {
                    message: format!("Invalid rule-based agent config: {}", e),
                    source: Some(Box::new(e)),
                })?;
            Ok(Box::new(RuleBasedAgent::new(
                config.name.clone(),
                config.role.clone(),
                rule_config,
            )))
        }
        AgentType::Retrieval => {
            use retrieval::{RetrievalAgent, RetrievalAgentConfig, InMemoryVectorDB, SimpleEmbeddingGenerator};
            let retrieval_config: RetrievalAgentConfig = serde_json::from_value(config.config.clone())
                .map_err(|e| MoaError::Config {
                    message: format!("Invalid retrieval agent config: {}", e),
                    source: Some(Box::new(e)),
                })?;
            Ok(Box::new(RetrievalAgent::new(
                config.name.clone(),
                config.role.clone(),
                retrieval_config,
                Box::new(InMemoryVectorDB::new()),
                Box::new(SimpleEmbeddingGenerator::new()),
            )))
        }
        AgentType::Custom(name) => {
            Err(MoaError::Config {
                message: format!("Custom agent '{}' not implemented", name),
                source: None,
            })
        }
    }
}

/// Create a self-MOA agent
pub fn create_self_moa(
    _id: String,
    inner_agent: Box<dyn Agent>,
    iterations: usize,
) -> Box<dyn Agent> {
    Box::new(SelfMoaAgent::new(Arc::from(inner_agent), iterations))
}

/// Create a sparse MOA agent
pub fn create_sparse_moa(
    id: String,
    name: String,
    description: String,
    capabilities: Vec<String>,
    config: AgentConfig,
    agents: Vec<Box<dyn Agent>>,
    max_concurrent: usize,
) -> MoaResult<Box<dyn Agent>> {
    Ok(Box::new(SparseMoaAgent::new(
        id,
        name,
        description,
        capabilities,
        config,
        agents,
        max_concurrent,
    )?) as Box<dyn Agent>)
}

/* // Commenting out CompositeAgent as it appears to be unused/incomplete
#[derive(Debug)]
pub struct CompositeAgent {
    base: BaseAgent,
    subagents: Vec<Box<dyn Agent>>,
    router: super::strategies::routing::RoutingStrategy, // field router is never read
}

impl CompositeAgent {
    pub fn new(
        id: String,
        name: String,
        description: String,
        capabilities: Vec<String>,
        config: AgentConfig,
        subagents: Vec<Box<dyn Agent>>,
        router: super::strategies::routing::RoutingStrategy,
    ) -> Self {
        Self {
            base: BaseAgent::new(id, name, description, capabilities, config),
            subagents,
            router,
        }
    }

    // methods get_agent_performance and combine_responses are never used
    /*
    fn get_agent_performance(&self) -> HashMap<String, f32> {
        let mut performance = HashMap::new();
        for agent in &self.subagents {
            let metrics = agent.get_metrics(); // Assuming get_metrics provides some performance score
            performance.insert(agent.get_id().to_string(), metrics.success_rate); // Example metric
        }
        performance
    }

    fn combine_responses(&self, responses: &[models::AgentResponse]) -> models::AgentResponse {
        // Simple combination: return the response with the highest confidence
        // More sophisticated logic would go here, potentially using the router
        responses
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
            .unwrap_or_else(|| {
                // Fallback if responses is empty or all confidences are NaN
                models::AgentResponse {
                    id: Uuid::new_v4().to_string(),
                    agent_id: self.get_id().to_string(),
                    request: models::MoaRequest { // This would need a default or passed request
                        id: Uuid::new_v4(),
                        query: "".to_string(),
                        context: None,
                        timestamp: Utc::now(),
                        metadata: HashMap::new(),
                    },
                    content: "No response combined".to_string(),
                    confidence: 0.0,
                    timestamp: Utc::now(),
                    metrics: models::ResponseMetrics::default(),
                }
            })
    }
    */
}

#[async_trait]
impl Agent for CompositeAgent {
    fn get_id(&self) -> &str {
        self.base.get_id()
    }
    
    fn get_name(&self) -> &str {
        self.base.get_name()
    }
    
    fn get_description(&self) -> &str {
        self.base.get_description()
    }
    
    fn get_capabilities(&self) -> &[String] {
        self.base.get_capabilities()
    }
    
    fn get_config(&self) -> &AgentConfig {
        self.base.get_config()
    }
    
    async fn process_request(&self, request: &models::MoaRequest) -> MoaResult<models::AgentResponse> {
        if self.subagents.is_empty() {
            return Err(MoaError::Agent {
                agent: self.get_id().to_string(),
                message: format!("CompositeAgent '{}': No subagents available to process request", self.get_id())
            });
        }

        // For simplicity, send to the first available sub-agent
        // In a real scenario, you might use a routing strategy here
        match self.subagents.get(0) {
            Some(agent) => agent.process_request(request).await,
            None => unreachable!("Checked for empty subagents already"),
        }
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
*/

/// Agent feedback for learning and adaptation
#[derive(Debug, Clone)]
pub struct AgentFeedback {
    /// Feedback score (0.0 to 1.0)
    pub score: f32,
    /// Feedback message
    pub message: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct Manager {
    agents: HashMap<String, Box<dyn Agent>>,
    metrics: HashMap<String, AgentMetrics>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            metrics: HashMap::new(),
        }
    }

    pub fn add_agent(&mut self, agent: Box<dyn Agent>) -> MoaResult<()> {
        let id = agent.get_id().to_string();
        if self.agents.contains_key(&id) {
            return Err(MoaError::Agent {
                agent_id: "Manager".to_string(),
                message: format!("Agent with ID '{}' already exists", id),
                source: None,
            });
        }
        self.metrics.insert(id.clone(), AgentMetrics::default());
        self.agents.insert(id, agent);
        Ok(())
    }

    pub fn remove_agent(&mut self, id: &str) -> MoaResult<()> {
        if !self.agents.contains_key(id) {
            return Err(MoaError::Agent {
                agent_id: "Manager".to_string(),
                message: format!("Agent with ID '{}' not found", id),
                source: None,
            });
        }
        self.agents.remove(id);
        self.metrics.remove(id);
        Ok(())
    }

    pub fn list_agents(&self) -> Vec<(String, AgentRole, f32)> {
        self.agents.iter()
            .map(|(id, agent)| {
                let metrics = agent.get_metrics();
                (id.clone(), agent.get_config().role.clone(), metrics.success_rate)
            })
            .collect()
    }

    pub fn get_agent(&self, id: &str) -> Option<&dyn Agent> {
        self.agents.get(id).map(|boxed_agent| boxed_agent.as_ref())
    }

    pub fn get_metrics(&self, id: &str) -> Option<AgentMetrics> {
        self.metrics.get(id).cloned()
    }
}