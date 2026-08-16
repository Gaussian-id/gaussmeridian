use crate::{
    agents::{Agent, BaseAgent, AgentMetrics},
    config::AgentConfig,
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest, ResponseMetrics},
    utils::WeightedSelector,
};
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, warn};
use uuid::Uuid;
use chrono::Utc;
use rand::thread_rng;

/// Sparse MOA agent that uses selective agent activation
#[derive(Debug)]
pub struct SparseMoaAgent {
    /// Base agent implementation
    base: BaseAgent,
    /// Available agents
    agents: Vec<Box<dyn Agent>>,
    /// Number of agents to activate
    k: usize,
    /// Selection strategy
    selection: WeightedSelector<usize>,
    /// Maximum concurrent requests
    max_concurrent: usize,
    /// Agent performance history
    history: Arc<RwLock<Vec<(AgentResponse, f32)>>>,
}

impl SparseMoaAgent {
    pub fn new(
        id: String,
        name: String,
        description: String,
        capabilities: Vec<String>,
        config: AgentConfig,
        agents: Vec<Box<dyn Agent>>,
        max_concurrent: usize,
    ) -> MoaResult<Self> {
        Ok(Self {
            base: BaseAgent::new(id, name, description, capabilities, config),
            agents,
            k: 3, // Default value
            max_concurrent,
            history: Arc::new(RwLock::new(Vec::new())),
            selection: WeightedSelector::new(Vec::new()),
        })
    }

    /// Select agents based on query characteristics
    async fn select_agents(&self, _request: &MoaRequest) -> MoaResult<Vec<usize>> {
        let mut selected = Vec::new();
        let history = self.history.read().await;
        let mut rng = thread_rng(); // Create RNG here

        // Simple selection based on historical performance
        let mut agent_scores: Vec<(usize, f32)> = (0..self.agents.len())
            .map(|i| (i, 0.0))
            .collect();

        for (response, confidence) in history.iter() {
            // Assuming response.request.id was meant to be response.agent_id for matching
            // This logic needs to be robust if agent_id is not directly in response or request.id isn't agent id.
            // For now, let's assume a placeholder or that AgentResponse has agent_id.
            if let Some(agent_idx) = self.agents.iter().position(|agent| agent.get_id() == response.agent_id) {
                 agent_scores[agent_idx].1 += confidence;
            }
        }

        // Sort by score and select top k
        agent_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        selected.extend(agent_scores.iter().take(self.k).map(|(i, _)| *i));

        // If not enough agents selected by score, use weighted random selection for the remainder
        if selected.len() < self.k {
            let num_to_select_randomly = self.k - selected.len();
            // Create a temporary list of items not already selected for weighted random selection
            let available_for_random: Vec<(usize, f64)> = self.agents.iter().enumerate()
                .filter(|(i, _)| !selected.contains(i))
                .map(|(i, _agent)| {
                    // Use some base weight, or perhaps a score from agent_scores if available
                    let weight = agent_scores.iter().find(|(idx, _)| *idx == i).map_or(1.0, |(_, score)| score.max(0.1) as f64); // Ensure non-zero positive weight
                    (i, weight)
                })
                .collect();

            if !available_for_random.is_empty() {
                let temp_selector = WeightedSelector::new(available_for_random);
                selected.extend(temp_selector.select_multiple(num_to_select_randomly, &mut rng));
            }
        }
        
        // Ensure unique agents if select_multiple could return duplicates (it shouldn't with remove)
        selected.sort_unstable();
        selected.dedup();

        Ok(selected)
    }

    /// Generate responses from selected agents
    async fn generate_responses(
        &self,
        selected_indices: &[usize],
        request: &MoaRequest,
    ) -> Vec<AgentResponse> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let mut futures = FuturesUnordered::new();

        for &idx in selected_indices {
            let agent = &self.agents[idx];
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let request = request.clone();

            futures.push(async move {
                let result = agent.process_request(&request).await;
                drop(permit);
                result
            });
        }

        let mut responses = Vec::new();
        while let Some(result) = futures.next().await {
            match result {
                Ok(response) => responses.push(response),
                Err(e) => warn!("Agent response error: {}", e),
            }
        }

        responses
    }

    /// Update agent performance history
    async fn update_history(&self, responses: &[AgentResponse]) {
        let mut history = self.history.write().await;
        
        for response in responses {
            history.push((response.clone(), response.confidence as f32));
            
            // Keep history size manageable
            if history.len() > 1000 {
                history.remove(0);
            }
        }
    }
}

#[async_trait]
impl Agent for SparseMoaAgent {
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

    async fn process_request(&self, request: &MoaRequest) -> MoaResult<AgentResponse> {
        let _start = std::time::Instant::now();

        // Select agents
        let selected_indices = self.select_agents(request).await?;
        debug!("Selected {} agents for request", selected_indices.len());

        // Generate responses
        let responses = self.generate_responses(&selected_indices, request).await;
        if responses.is_empty() {
            return Err(MoaError::agent(
                "No successful responses from selected agents".to_string(),
                self.get_id().to_string(),
                None::<Box<dyn std::error::Error + Send + Sync>>
            ));
        }

        // Update history
        self.update_history(&responses).await;

        // Combine responses
        let mut combined_content = String::new();
        let mut total_confidence = 0.0;
        
        for response in &responses {
            combined_content.push_str(&response.content);
            combined_content.push_str("\n\n");
            total_confidence += response.confidence;
        }
        
        let avg_confidence = total_confidence / responses.len() as f64;

        Ok(AgentResponse {
            id: Uuid::new_v4().to_string(),
            agent_id: self.base.get_id().to_string(),
            request: request.clone(),
            content: combined_content,
            confidence: avg_confidence,
            timestamp: Utc::now(),
            metrics: ResponseMetrics::default(),
        })
    }

    fn update_config(&mut self, config: AgentConfig) -> MoaResult<()> {
        self.base.update_config(config)
    }

    fn get_metrics(&self) -> AgentMetrics {
        self.base.get_metrics()
    }

    fn reset(&mut self) -> MoaResult<()> {
        // Reset base agent
        self.base.reset()?;
        
        // Reset history
        if let Ok(mut history) = self.history.try_write() {
            history.clear();
        }
        
        Ok(())
    }
}