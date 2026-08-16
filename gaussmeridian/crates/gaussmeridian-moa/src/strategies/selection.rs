use crate::{
    agents::Agent,
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaResponse, ResponseMetrics},
    metrics,
};
use async_trait::async_trait;
use linfa::prelude::*;
use linfa_clustering::KMeans;
use ndarray::{Array1, Array2};
use rand::{thread_rng, Rng, seq::SliceRandom};
use std::time::Instant;
use tracing::{debug, info};
use uuid::Uuid;
use chrono::Utc;

/// Selection strategy for choosing agents
pub struct SelectionStrategy {
    /// Number of agents to select
    n_agents: usize,
    /// Selection method
    method: SelectionMethod,
}

/// Available selection methods
#[derive(Debug, Clone)]
pub enum SelectionMethod {
    /// Select top-k agents by confidence
    TopK,
    /// Select agents using k-means clustering
    Clustering,
    /// Select agents randomly
    Random,
}

impl SelectionStrategy {
    pub fn new(n_agents: usize, method: SelectionMethod) -> Self {
        Self {
            n_agents,
            method,
        }
    }
    
    async fn select_responses(
        &self,
        responses: &[AgentResponse]
    ) -> MoaResult<Vec<AgentResponse>> {
        match self.method {
            SelectionMethod::TopK => self.select_top_k(responses),
            SelectionMethod::Clustering => self.select_clustering(responses).await,
            SelectionMethod::Random => self.select_random(responses),
        }
    }

    fn select_top_k(&self, responses: &[AgentResponse]) -> MoaResult<Vec<AgentResponse>> {
        let mut selected = responses.to_vec();
        selected.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        selected.truncate(self.n_agents);
        Ok(selected)
    }

    async fn select_clustering(&self, responses: &[AgentResponse]) -> MoaResult<Vec<AgentResponse>> {
        if responses.len() <= self.n_agents {
            return Ok(responses.to_vec());
        }

        // Convert responses to embeddings
        let embeddings = self.responses_to_embeddings(responses)?;
        
        // Create dataset
        let dataset = DatasetBase::new(embeddings, (0..responses.len()).collect::<Vec<_>>());

        // Run k-means clustering
        let kmeans = KMeans::params(self.n_agents)
            .max_n_iterations(100)
            .tolerance(1e-4)
            .fit(&dataset)
            .map_err(|e| MoaError::Strategy(format!("Failed to fit k-means: {}", e)))?;

        // Get cluster assignments
        let assignments = kmeans.predict(&dataset);

        // Select best response from each cluster
        let mut selected = Vec::new();
        for cluster in 0..self.n_agents {
            let cluster_responses: Vec<&AgentResponse> = assignments.iter()
                .enumerate()
                .filter(|(_, &c)| c == cluster)
                .map(|(i, _)| &responses[i])
                .collect();

            if let Some(best) = cluster_responses.iter()
                .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap()) {
                selected.push((*best).clone());
            }
        }

        Ok(selected)
    }

    fn select_random(&self, responses: &[AgentResponse]) -> MoaResult<Vec<AgentResponse>> {
        let mut rng = thread_rng();
        let mut selected = responses.to_vec();
        selected.shuffle(&mut rng);
        selected.truncate(self.n_agents);
        Ok(selected)
    }

    fn responses_to_embeddings(&self, responses: &[AgentResponse]) -> MoaResult<Array2<f64>> {
        let mut embeddings = Array2::zeros((responses.len(), 768));
        
        for (i, response) in responses.iter().enumerate() {
            let embedding = self.text_to_embedding(&response.content)?;
            embeddings.row_mut(i).assign(&embedding);
        }
        
        Ok(embeddings)
    }

    fn text_to_embedding(&self, text: &str) -> MoaResult<Array1<f64>> {
        let mut embedding = Array1::zeros(768);
        let mut count = 0;
        
        for (i, _word) in text.split_whitespace().enumerate() {
            // Simple bag-of-words embedding for demonstration
            embedding[i % 768] += 1.0;
            count += 1;
        }
        
        if count > 0 {
            embedding /= count as f64;
        }
        
        // Normalize embedding
        let norm = embedding.dot(&embedding).sqrt();
        if norm > 0.0 {
            embedding /= norm;
        }
        
        Ok(embedding)
    }
}

#[async_trait]
impl super::MoaStrategy for SelectionStrategy {
    async fn process(
        &self,
        _agents: &[Box<dyn Agent>],
        responses: &[AgentResponse],
    ) -> MoaResult<MoaResponse> {
        let start = Instant::now();
        
        // Select responses
        let selected_responses = self.select_responses(responses).await?;
        
        // Combine selected responses
        let mut combined_content = String::new();
        let mut total_confidence = 0.0;
        
        for response in &selected_responses {
            combined_content.push_str(&response.content);
            combined_content.push_str("\n\n");
            total_confidence += response.confidence;
        }
        
        let avg_confidence = if !selected_responses.is_empty() {
            total_confidence / selected_responses.len() as f32
        } else {
            0.0
        };

        // Record metrics
        metrics::record_request_latency(
            "selection",
            "process",
            start.elapsed().as_millis() as u64
        );

        Ok(MoaResponse {
            id: Uuid::new_v4(),
            content: combined_content,
            confidence: avg_confidence,
            agent_responses: selected_responses,
            timestamp: Utc::now(),
            metrics: ResponseMetrics::default(),
        })
    }

    fn name(&self) -> &str {
        "selection"
    }
}