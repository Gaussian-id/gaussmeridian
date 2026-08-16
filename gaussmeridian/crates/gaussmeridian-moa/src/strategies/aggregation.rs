use crate::{
    agents::Agent,
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest, MoaResponse, ResponseMetrics},
    utils::WeightedSelector,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;
use chrono::{Utc, DateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorProfile {
    /// Supported task types
    pub task_types: Vec<String>,
    /// Performance metrics per task type
    pub performance: HashMap<String, Vec<f32>>,
    /// Specialization areas
    pub specializations: Vec<String>,
    /// Resource requirements
    pub resource_requirements: ResourceRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_memory_mb: usize,
    pub expected_latency_ms: u64,
    pub max_input_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCharacteristics {
    pub task_type: String,
    pub complexity: f32,
    pub expected_disagreement: f32,
    pub domain: String,
}

pub struct AdaptiveAggregator {
    /// Available aggregation methods
    aggregators: HashMap<String, Box<dyn Aggregator>>,
    /// Aggregator profiles
    profiles: Arc<RwLock<HashMap<String, AggregatorProfile>>>,
    /// Performance history
    history: Arc<RwLock<HashMap<String, Vec<AggregationMetrics>>>>,
    /// Task characteristics extractor
    task_analyzer: TaskAnalyzer,
}

#[derive(Debug, Clone, Copy)]
pub enum AggregationMethod {
    Equal,
    Weighted,
    Confidence,
}

pub struct AggregationStrategy {
    method: AggregationMethod,
    confidence_threshold: f32,
    history: Arc<RwLock<Vec<(AgentResponse, f32)>>>,
}

impl AggregationStrategy {
    pub fn new(method: AggregationMethod, confidence_threshold: f32) -> Self {
        Self {
            method,
            confidence_threshold,
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
pub trait Aggregator: Send + Sync {
    async fn aggregate(&self, responses: &[AgentResponse]) -> MoaResult<MoaResponse>;
    fn name(&self) -> &str;
}

impl AdaptiveAggregator {
    pub fn new() -> Self {
        Self {
            aggregators: HashMap::new(),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(HashMap::new())),
            task_analyzer: TaskAnalyzer::new(),
        }
    }

    /// Register a new aggregator with its profile
    pub async fn register_aggregator(
        &mut self,
        name: String,
        aggregator: Box<dyn Aggregator>,
        profile: AggregatorProfile,
    ) {
        self.aggregators.insert(name.clone(), aggregator);
        self.profiles.write().await.insert(name, profile);
    }

    /// Select best aggregator for the task
    async fn select_aggregator(
        &self,
        request: &MoaRequest,
        responses: &[AgentResponse],
    ) -> MoaResult<String> {
        let characteristics = self.task_analyzer.analyze_task(request, responses).await?;
        let profiles = self.profiles.read().await;
        let history = self.history.read().await;

        let mut best_score = f32::NEG_INFINITY;
        let mut selected = None;

        for (name, profile) in profiles.iter() {
            let score = self.compute_aggregator_score(
                name,
                profile,
                &characteristics,
                history.get(name),
            ).await;

            if score > best_score {
                best_score = score;
                selected = Some(name.clone());
            }
        }

        selected.ok_or_else(|| MoaError::Strategy("No suitable aggregator found".to_string()))
    }

    /// Compute aggregator score for task
    async fn compute_aggregator_score(
        &self,
        name: &str,
        profile: &AggregatorProfile,
        task: &TaskCharacteristics,
        history: Option<&Vec<AggregationMetrics>>,
    ) -> f32 {
        let mut score = 0.0;

        // Task type match
        if profile.task_types.contains(&task.task_type) {
            score += 1.0;
        }

        // Domain specialization
        if profile.specializations.contains(&task.domain) {
            score += 1.0;
        }

        // Historical performance
        if let Some(metrics) = history {
            let task_metrics: Vec<_> = metrics
                .iter()
                .filter(|m| m.task_type == task.task_type)
                .collect();

            if !task_metrics.is_empty() {
                let avg_quality = task_metrics.iter().map(|m| m.quality_score).sum::<f32>() 
                    / task_metrics.len() as f32;
                score += avg_quality;
            }
        }

        // Complexity handling
        if task.complexity > 0.7 {
            // Prefer more sophisticated aggregators for complex tasks
            score += if profile.specializations.len() > 2 { 0.5 } else { 0.0 };
        }

        // Disagreement handling
        if task.expected_disagreement > 0.5 {
            // Prefer aggregators with conflict resolution capabilities
            score += if profile.task_types.contains(&String::from("conflict_resolution")) { 0.5 } else { 0.0 };
        }

        score
    }

    /// Aggregate responses using the best aggregator
    pub async fn aggregate(
        &self,
        request: &MoaRequest,
        responses: &[AgentResponse],
    ) -> MoaResult<MoaResponse> {
        let aggregator_name = self.select_aggregator(request, responses).await?;
        let aggregator = self.aggregators.get(&aggregator_name)
            .ok_or_else(|| MoaError::Strategy("Selected aggregator not found".to_string()))?;

        let start = std::time::Instant::now();
        let result = aggregator.aggregate(responses).await?;
        let duration = start.elapsed();

        // Record metrics
        let mut history = self.history.write().await;
        let metrics = history.entry(aggregator_name).or_insert_with(Vec::new);
        metrics.push(AggregationMetrics {
            task_type: self.task_analyzer.analyze_task(request, responses).await?.task_type,
            quality_score: result.confidence,
            latency_ms: duration.as_millis() as u64,
            success: result.confidence > 0.5,
        });

        Ok(result)
    }
}

#[derive(Debug)]
struct TaskAnalyzer {
    /// Task type classifier
    classifier: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl TaskAnalyzer {
    fn new() -> Self {
        Self {
            classifier: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn analyze_task(
        &self,
        request: &MoaRequest,
        responses: &[AgentResponse],
    ) -> MoaResult<TaskCharacteristics> {
        let task_type = self.classify_task(request).await;
        let complexity = self.estimate_complexity(request, responses);
        let expected_disagreement = self.estimate_disagreement(responses);
        let domain = self.detect_domain(request);

        Ok(TaskCharacteristics {
            task_type,
            complexity,
            expected_disagreement,
            domain,
        })
    }

    async fn classify_task(&self, request: &MoaRequest) -> String {
        let query = request.query.to_lowercase();
        let classifier = self.classifier.read().await;

        for (task_type, keywords) in classifier.iter() {
            if keywords.iter().any(|k| query.contains(k)) {
                return task_type.clone();
            }
        }

        "general".to_string()
    }

    fn estimate_complexity(&self, request: &MoaRequest, responses: &[AgentResponse]) -> f32 {
        let query_length = request.query.len();
        let avg_response_length = responses.iter()
            .map(|r| r.content.len())
            .sum::<usize>() as f32 / responses.len() as f32;

        // Normalize complexity score between 0 and 1
        ((query_length as f32 / 1000.0) + (avg_response_length / 5000.0)) / 2.0
    }

    fn estimate_disagreement(&self, responses: &[AgentResponse]) -> f32 {
        if responses.len() <= 1 {
            return 0.0;
        }

        let avg_confidence = responses.iter()
            .map(|r| r.confidence)
            .sum::<f32>() / responses.len() as f32;

        let variance = responses.iter()
            .map(|r| (r.confidence - avg_confidence).powi(2))
            .sum::<f32>() / responses.len() as f32;

        (variance.sqrt() / avg_confidence).min(1.0)
    }

    fn detect_domain(&self, request: &MoaRequest) -> String {
        let query = request.query.to_lowercase();
        
        let domain_keywords = vec![
            ("math", vec!["math", "calculation", "equation"]),
            ("science", vec!["physics", "chemistry", "biology"]),
            ("programming", vec!["code", "programming", "software"]),
            ("medicine", vec!["medical", "health", "disease"]),
        ];

        for (domain, keywords) in domain_keywords {
            if keywords.iter().any(|k| query.contains(k)) {
                return domain.to_string();
            }
        }

        "general".to_string()
    }
}

#[derive(Debug)]
struct AggregationMetrics {
    task_type: String,
    quality_score: f32,
    latency_ms: u64,
    success: bool,
}

#[async_trait]
impl MoaStrategy for AggregationStrategy {
    async fn process(
        &self,
        _agents: &[Box<dyn Agent>],
        responses: &[AgentResponse],
    ) -> MoaResult<MoaResponse> {
        if responses.is_empty() {
            return Err(MoaError::Strategy("No responses to process".to_string()));
        }

        // Compute weights for each response
        let mut weights = Vec::new();
        let mut total_weight = 0.0;

        for response in responses {
            let weight = match self.method {
                AggregationMethod::Weighted => response.confidence,
                AggregationMethod::Equal => 1.0,
                AggregationMethod::Confidence => {
                    if response.confidence >= self.confidence_threshold {
                        response.confidence
                    } else {
                        0.0
                    }
                }
            };
            weights.push(weight);
            total_weight += weight;
        }

        // Normalize weights
        if total_weight > 0.0 {
            for weight in &mut weights {
                *weight /= total_weight;
            }
        }

        // Update history
        let mut history = self.history.write().await;
        for (response, weight) in responses.iter().zip(weights.iter()) {
            history.push((response.clone(), *weight));
        }

        // Combine responses
        let mut combined_content = String::new();
        let mut total_confidence = 0.0;

        for (response, weight) in responses.iter().zip(weights.iter()) {
            combined_content.push_str(&response.content);
            combined_content.push_str("\n\n");
            total_confidence += response.confidence * weight;
        }

        Ok(MoaResponse {
            id: Uuid::new_v4(),
            content: combined_content,
            confidence: total_confidence,
            agent_responses: responses.to_vec(),
            timestamp: Utc::now(),
            metrics: ResponseMetrics::default(),
        })
    }

    fn name(&self) -> &str {
        "aggregation"
    }
} 