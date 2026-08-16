use crate::{
    agents::Agent,
    error::{MoaError, MoaResult},
    models::{AgentResponse, MoaRequest, MoaResponse},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, info};
use futures::future::FuturesUnordered;
use std::sync::Semaphore;
use std::time::{Instant};
use num_cpus;
use sys_info;
use rand;
use num_traits;
use rand::Rng;
use rand_distr;

/// ReConcile strategy for consensus building and knowledge reconciliation
#[derive(Debug)]
pub struct ReConcileStrategy {
    /// Maximum rounds for reconciliation
    max_rounds: usize,
    /// Minimum confidence threshold
    min_confidence: f32,
    /// Consensus threshold
    consensus_threshold: f32,
    /// Knowledge base for reconciliation
    knowledge_base: Arc<RwLock<KnowledgeBase>>,
    /// Reconciliation history
    history: Arc<RwLock<HashMap<String, Vec<ReconciliationRound>>>>,
}

/// Knowledge base for reconciliation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeBase {
    /// Domain-specific knowledge
    domain_knowledge: HashMap<String, Vec<KnowledgeEntry>>,
    /// Consensus patterns
    consensus_patterns: HashMap<String, ConsensusPattern>,
    /// Conflict resolution rules
    resolution_rules: Vec<ResolutionRule>,
}

/// Knowledge entry in the knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeEntry {
    /// Entry content
    content: String,
    /// Source agent
    source: String,
    /// Confidence score
    confidence: f32,
    /// Verification status
    verified: bool,
    /// Supporting evidence
    evidence: Vec<String>,
}

/// Consensus pattern for reconciliation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsensusPattern {
    /// Pattern description
    description: String,
    /// Required agreement level
    required_agreement: f32,
    /// Supporting agents count
    min_supporting_agents: usize,
    /// Domain applicability
    applicable_domains: Vec<String>,
}

/// Resolution rule for conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResolutionRule {
    /// Rule condition
    condition: String,
    /// Resolution action
    action: String,
    /// Priority level
    priority: u32,
    /// Success rate
    success_rate: f32,
}

/// Reconciliation round information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationRound {
    /// Round number
    pub round: usize,
    /// Agent responses
    pub responses: Vec<AgentResponse>,
    /// Identified conflicts
    pub conflicts: Vec<Conflict>,
    /// Resolution attempts
    pub resolutions: Vec<Resolution>,
    /// Consensus status
    pub consensus_reached: bool,
    /// Round metrics
    pub metrics: RoundMetrics,
}

/// Conflict in reconciliation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Conflict description
    pub description: String,
    /// Involved agents
    pub agents: Vec<String>,
    /// Conflicting statements
    pub statements: Vec<String>,
    /// Conflict type
    pub conflict_type: ConflictType,
    /// Resolution status
    pub resolved: bool,
}

/// Type of conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    /// Factual disagreement
    Factual,
    /// Logical contradiction
    Logical,
    /// Perspective difference
    Perspective,
    /// Incomplete information
    Incomplete,
}

/// Resolution of conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    /// Resolution description
    pub description: String,
    /// Applied rule
    pub rule: String,
    /// Supporting evidence
    pub evidence: Vec<String>,
    /// Resolution confidence
    pub confidence: f32,
    /// Accepted by agents
    pub accepted_by: Vec<String>,
}

/// Round metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundMetrics {
    /// Agreement level
    pub agreement_level: f32,
    /// Number of conflicts
    pub conflict_count: usize,
    /// Resolution success rate
    pub resolution_rate: f32,
    /// Round duration
    pub duration_ms: u64,
}

/// Generate random values in the given range
fn generate_random_values(min: f64, max: f64, size: usize) -> Vec<f64> {
    let mut rng = rand::thread_rng();
    (0..size)
        .map(|_| rng.gen_range(min..max))
        .collect()
}

/// Generate random values with normal distribution
fn generate_random_normal(mean: f64, std: f64, size: usize) -> Vec<f64> {
    use rand_distr::{Normal, Distribution};
    let normal = Normal::new(mean, std).unwrap();
    let mut rng = rand::thread_rng();
    (0..size)
        .map(|_| normal.sample(&mut rng))
        .collect()
}

/// Convert f64 to f32 with clamping
fn to_f32_clamped(x: f64) -> f32 {
    if x.is_nan() {
        0.0
    } else if x.is_infinite() {
        if x.is_sign_positive() {
            f32::MAX
        } else {
            f32::MIN
        }
    } else {
        x as f32
    }
}

impl ReConcileStrategy {
    /// Create a new ReConcile strategy
    pub fn new(
        max_rounds: usize,
        min_confidence: f32,
        consensus_threshold: f32,
    ) -> Self {
        Self {
            max_rounds,
            min_confidence,
            consensus_threshold,
            knowledge_base: Arc::new(RwLock::new(KnowledgeBase {
                domain_knowledge: HashMap::new(),
                consensus_patterns: HashMap::new(),
                resolution_rules: Vec::new(),
            })),
            history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new ReConcile strategy with default rules
    pub fn new_with_defaults(
        max_rounds: usize,
        min_confidence: f32,
        consensus_threshold: f32,
    ) -> Self {
        let mut strategy = Self::new(max_rounds, min_confidence, consensus_threshold);
        
        // Initialize default resolution rules
        let default_rules = vec![
            ResolutionRule {
                condition: "factual_contradiction".into(),
                action: "verify_against_knowledge_base".into(),
                priority: 1,
                success_rate: 0.9,
            },
            ResolutionRule {
                condition: "logical_inconsistency".into(),
                action: "apply_logical_reasoning".into(),
                priority: 2,
                success_rate: 0.85,
            },
            ResolutionRule {
                condition: "temporal_conflict".into(),
                action: "use_most_recent_data".into(),
                priority: 3,
                success_rate: 0.95,
            },
            ResolutionRule {
                condition: "perspective_difference".into(),
                action: "combine_perspectives".into(),
                priority: 4,
                success_rate: 0.8,
            },
            ResolutionRule {
                condition: "incomplete_information".into(),
                action: "request_clarification".into(),
                priority: 5,
                success_rate: 0.75,
            },
        ];

        // Initialize default consensus patterns
        let mut consensus_patterns = HashMap::new();
        consensus_patterns.insert(
            "unanimous".into(),
            ConsensusPattern {
                description: "All agents agree on the response".into(),
                required_agreement: 1.0,
                min_supporting_agents: 2,
                applicable_domains: vec!["critical", "financial", "medical"].into_iter().map(String::from).collect(),
            },
        );
        consensus_patterns.insert(
            "majority".into(),
            ConsensusPattern {
                description: "Majority of agents agree".into(),
                required_agreement: 0.67,
                min_supporting_agents: 2,
                applicable_domains: vec!["general", "opinion", "recommendation"].into_iter().map(String::from).collect(),
            },
        );
        consensus_patterns.insert(
            "expert_weighted".into(),
            ConsensusPattern {
                description: "Expert opinions weighted more heavily".into(),
                required_agreement: 0.8,
                min_supporting_agents: 1,
                applicable_domains: vec!["technical", "scientific", "specialized"].into_iter().map(String::from).collect(),
            },
        );

        // Initialize knowledge base with domain-specific entries
        let mut domain_knowledge = HashMap::new();
        domain_knowledge.insert(
            "technical".into(),
            vec![
                KnowledgeEntry {
                    content: "Programming best practices and patterns".into(),
                    source: "expert_system".into(),
                    confidence: 0.95,
                    verified: true,
                    evidence: vec!["documented_standards".into(), "peer_review".into()],
                },
            ],
        );
        domain_knowledge.insert(
            "scientific".into(),
            vec![
                KnowledgeEntry {
                    content: "Scientific method and verification procedures".into(),
                    source: "scientific_database".into(),
                    confidence: 0.98,
                    verified: true,
                    evidence: vec!["peer_reviewed_papers".into(), "experimental_data".into()],
                },
            ],
        );

        // Set up the knowledge base
        let kb = KnowledgeBase {
            domain_knowledge,
            consensus_patterns,
            resolution_rules: default_rules,
        };

        *strategy.knowledge_base.write().blocking_lock() = kb;
        strategy
    }

    /// Start a reconciliation round
    pub async fn start_round(
        &self,
        reconciliation_id: &str,
        request: &MoaRequest,
        agents: &[Box<dyn Agent>],
    ) -> MoaResult<Vec<AgentResponse>> {
        let start_time = std::time::Instant::now();
        let mut responses = Vec::new();

        // Generate initial responses
        for agent in agents {
            if let Ok(response) = agent.generate_response(request).await {
                if response.confidence >= self.min_confidence {
                    responses.push(response);
                }
            }
        }

        // Identify conflicts
        let conflicts = self.identify_conflicts(&responses).await?;

        // Attempt conflict resolution
        let resolutions = self.resolve_conflicts(&conflicts, &responses).await?;

        // Update knowledge base
        self.update_knowledge_base(&responses, &resolutions).await?;

        // Check consensus
        let agreement_level = self.calculate_agreement_level(&responses).await;
        let consensus_reached = agreement_level >= self.consensus_threshold;

        // Record round
        let round_metrics = RoundMetrics {
            agreement_level,
            conflict_count: conflicts.len(),
            resolution_rate: if conflicts.is_empty() {
                1.0
            } else {
                resolutions.len() as f32 / conflicts.len() as f32
            },
            duration_ms: start_time.elapsed().as_millis() as u64,
        };

        let mut history = self.history.write().await;
        let rounds = history.entry(reconciliation_id.to_string()).or_insert_with(Vec::new);
        
        rounds.push(ReconciliationRound {
            round: rounds.len(),
            responses: responses.clone(),
            conflicts,
            resolutions,
            consensus_reached,
            metrics: round_metrics,
        });

        Ok(responses)
    }

    /// Identify conflicts in responses
    async fn identify_conflicts(&self, responses: &[AgentResponse]) -> MoaResult<Vec<Conflict>> {
        let mut conflicts = Vec::new();
        let kb = self.knowledge_base.read().await;

        // Compare responses pairwise
        for (i, r1) in responses.iter().enumerate() {
            for r2 in responses.iter().skip(i + 1) {
                // Check for factual conflicts
                if let Some(conflict) = self.check_factual_conflict(r1, r2, &kb).await {
                    conflicts.push(conflict);
                }

                // Check for logical conflicts
                if let Some(conflict) = self.check_logical_conflict(r1, r2, &kb).await {
                    conflicts.push(conflict);
                }

                // Check for perspective conflicts
                if let Some(conflict) = self.check_perspective_conflict(r1, r2, &kb).await {
                    conflicts.push(conflict);
                }
            }
        }

        Ok(conflicts)
    }

    /// Check for factual conflicts with enhanced verification
    async fn check_factual_conflict(
        &self,
        r1: &AgentResponse,
        r2: &AgentResponse,
        kb: &KnowledgeBase,
    ) -> Option<Conflict> {
        let content1 = r1.content.to_lowercase();
        let content2 = r2.content.to_lowercase();

        // Extract numerical claims
        let numbers1 = extract_numerical_claims(&content1);
        let numbers2 = extract_numerical_claims(&content2);

        // Check for numerical contradictions
        for (claim1, value1) in &numbers1 {
            if let Some(value2) = numbers2.get(claim1) {
                if (value1 - value2).abs() > 0.001 {
                    return Some(Conflict {
                        description: format!("Numerical contradiction found: {} vs {}", value1, value2),
                        agents: vec![r1.agent_id.clone(), r2.agent_id.clone()],
                        statements: vec![content1.clone(), content2.clone()],
                        conflict_type: ConflictType::Factual,
                        resolved: false,
                    });
                }
            }
        }

        // Check against knowledge base
        for (domain, entries) in &kb.domain_knowledge {
            for entry in entries {
                if content1.contains(&entry.content.to_lowercase()) 
                    && content2.contains(&entry.content.to_lowercase())
                    && content1 != content2 {
                    return Some(Conflict {
                        description: format!("Factual conflict in domain {}", domain),
                        agents: vec![r1.agent_id.clone(), r2.agent_id.clone()],
                        statements: vec![content1, content2],
                        conflict_type: ConflictType::Factual,
                        resolved: false,
                    });
                }
            }
        }

        None
    }

    /// Check for logical conflicts
    async fn check_logical_conflict(
        &self,
        r1: &AgentResponse,
        r2: &AgentResponse,
        kb: &KnowledgeBase,
    ) -> Option<Conflict> {
        // Implementation would check for logical contradictions
        None // Placeholder
    }

    /// Check for perspective conflicts
    async fn check_perspective_conflict(
        &self,
        r1: &AgentResponse,
        r2: &AgentResponse,
        kb: &KnowledgeBase,
    ) -> Option<Conflict> {
        // Implementation would identify differing perspectives
        None // Placeholder
    }

    /// Resolve identified conflicts
    async fn resolve_conflicts(
        &self,
        conflicts: &[Conflict],
        responses: &[AgentResponse],
    ) -> MoaResult<Vec<Resolution>> {
        let mut resolutions = Vec::new();
        let kb = self.knowledge_base.read().await;

        for conflict in conflicts {
            if let Some(resolution) = self.apply_resolution_rules(conflict, &kb.resolution_rules).await {
                resolutions.push(resolution);
            }
        }

        Ok(resolutions)
    }

    /// Apply resolution rules with enhanced logic
    async fn apply_resolution_rules(
        &self,
        conflict: &Conflict,
        rules: &[ResolutionRule],
    ) -> Option<Resolution> {
        // Sort rules by priority and success rate
        let mut applicable_rules: Vec<_> = rules.iter()
            .filter(|r| matches_conflict_type(conflict, &r.condition))
            .collect();
        applicable_rules.sort_by_key(|r| (r.priority, (r.success_rate * 100.0) as u32));

        if let Some(rule) = applicable_rules.first() {
            let (description, evidence) = match rule.action.as_str() {
                "verify_against_knowledge_base" => {
                    let kb = self.knowledge_base.read().await;
                    verify_against_kb(conflict, &kb)
                },
                "apply_logical_reasoning" => {
                    apply_logical_reasoning(conflict)
                },
                "use_most_recent_data" => {
                    use_most_recent_data(conflict)
                },
                "combine_perspectives" => {
                    combine_perspectives(conflict)
                },
                "request_clarification" => {
                    request_clarification(conflict)
                },
                _ => (
                    format!("Applied general resolution: {}", rule.action),
                    vec!["general_resolution".into()]
                ),
            };

            Some(Resolution {
                description,
                rule: rule.action.clone(),
                evidence,
                confidence: rule.success_rate,
                accepted_by: vec![],
            })
        } else {
            None
        }
    }

    /// Update knowledge base with new information
    async fn update_knowledge_base(
        &self,
        responses: &[AgentResponse],
        resolutions: &[Resolution],
    ) -> MoaResult<()> {
        let mut kb = self.knowledge_base.write().await;
        
        // Update with new knowledge from responses
        for response in responses {
            // Implementation would extract and store knowledge
        }

        // Update with successful resolutions
        for resolution in resolutions {
            if resolution.confidence >= self.min_confidence {
                // Implementation would update knowledge base
            }
        }

        Ok(())
    }

    /// Calculate agreement level between responses
    async fn calculate_agreement_level(&self, responses: &[AgentResponse]) -> f32 {
        if responses.is_empty() {
            return 0.0;
        }

        // Implementation would calculate semantic agreement level
        0.5 // Placeholder
    }

    /// Get reconciliation history
    pub async fn get_history(&self, reconciliation_id: &str) -> Option<Vec<ReconciliationRound>> {
        self.history.read().await.get(reconciliation_id).cloned()
    }

    /// Reset strategy state
    pub async fn reset(&self) -> MoaResult<()> {
        let mut kb = self.knowledge_base.write().await;
        let mut history = self.history.write().await;

        kb.domain_knowledge.clear();
        kb.consensus_patterns.clear();
        kb.resolution_rules.clear();
        history.clear();

        Ok(())
    }

    /// Process a batch of requests efficiently
    pub async fn process_batch(
        &self,
        requests: &[MoaRequest],
        agents: &[Box<dyn Agent>],
        batch_size: usize,
    ) -> MoaResult<Vec<MoaResponse>> {
        let mut responses = Vec::with_capacity(requests.len());
        let mut batches = Vec::new();

        // Group requests into batches
        for chunk in requests.chunks(batch_size) {
            batches.push(chunk.to_vec());
        }

        // Process batches concurrently
        let mut futures = FuturesUnordered::new();
        let semaphore = Arc::new(Semaphore::new(4)); // Limit concurrent batches

        for batch in batches {
            let agents = agents.to_vec();
            let strategy = self.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            futures.push(tokio::spawn(async move {
                let mut batch_responses = Vec::new();
                
                for request in batch {
                    let reconciliation_id = uuid::Uuid::new_v4().to_string();
                    let mut round = 0;
                    let mut final_response = None;

                    while round < strategy.max_rounds {
                        let round_responses = strategy.start_round(&reconciliation_id, &request, &agents).await?;
                        
                        if let Some(history) = strategy.get_history(&reconciliation_id).await {
                            if let Some(last_round) = history.last() {
                                if last_round.consensus_reached {
                                    final_response = Some(MoaResponse {
                                        content: round_responses.iter()
                                            .map(|r| r.content.clone())
                                            .collect::<Vec<_>>()
                                            .join("\n"),
                                        confidence: round_responses.iter()
                                            .map(|r| r.confidence)
                                            .sum::<f32>() / round_responses.len() as f32,
                                        agent_responses: round_responses,
                                    });
                                    break;
                                }
                            }
                        }
                        round += 1;
                    }

                    if let Some(response) = final_response {
                        batch_responses.push(Ok(response));
                    } else {
                        batch_responses.push(Err(MoaError::Strategy(
                            "Failed to reach consensus".into()
                        )));
                    }
                }

                drop(permit); // Release semaphore
                batch_responses
            }));
        }

        // Collect results
        while let Some(result) = futures.next().await {
            match result {
                Ok(batch_results) => {
                    for result in batch_results? {
                        responses.push(result?);
                    }
                }
                Err(e) => {
                    error!("Batch processing error: {}", e);
                    return Err(MoaError::Strategy(format!("Batch processing failed: {}", e)));
                }
            }
        }

        Ok(responses)
    }

    /// Optimize resource usage for a batch of requests
    pub async fn optimize_batch(
        &self,
        requests: &[MoaRequest],
        agents: &[Box<dyn Agent>],
    ) -> MoaResult<(Vec<MoaResponse>, ResourceMetrics)> {
        let start_time = Instant::now();
        let batch_size = self.calculate_optimal_batch_size(requests.len(), agents.len());
        
        let responses = self.process_batch(requests, agents, batch_size).await?;
        
        let metrics = ResourceMetrics {
            total_time_ms: start_time.elapsed().as_millis() as u64,
            requests_processed: requests.len(),
            agents_used: agents.len(),
            avg_response_time_ms: start_time.elapsed().as_millis() as u64 / requests.len() as u64,
            batch_size,
        };

        Ok((responses, metrics))
    }

    /// Calculate optimal batch size based on workload
    fn calculate_optimal_batch_size(&self, num_requests: usize, num_agents: usize) -> usize {
        let cpu_cores = num_cpus::get();
        let memory_gb = sys_info::mem_info().map(|m| m.total / 1024 / 1024).unwrap_or(8);
        
        // Heuristic formula for batch size:
        // - Consider available CPU cores
        // - Consider available memory
        // - Consider number of agents
        let base_size = (cpu_cores * 2).min(num_requests);
        let mem_factor = (memory_gb as f32 / num_agents as f32).sqrt() as usize;
        
        base_size.min(mem_factor).max(1)
    }
}

#[derive(Debug)]
pub struct ResourceMetrics {
    pub total_time_ms: u64,
    pub requests_processed: usize,
    pub agents_used: usize,
    pub avg_response_time_ms: u64,
    pub batch_size: usize,
}

#[async_trait]
impl super::MoaStrategy for ReConcileStrategy {
    async fn process(
        &self,
        agents: &[Box<dyn Agent>],
        responses: &[AgentResponse],
    ) -> MoaResult<MoaResponse> {
        if responses.is_empty() {
            return Err(MoaError::Strategy("No responses to process".into()));
        }

        let reconciliation_id = uuid::Uuid::new_v4().to_string();
        let request = &responses[0].request;
        let mut round = 0;
        let mut final_responses = responses.to_vec();

        while round < self.max_rounds {
            let round_responses = self.start_round(&reconciliation_id, request, agents).await?;
            
            // Check if consensus reached
            if let Some(history) = self.get_history(&reconciliation_id).await {
                if let Some(last_round) = history.last() {
                    if last_round.consensus_reached {
                        final_responses = round_responses;
                        break;
                    }
                }
            }

            round += 1;
        }

        // Combine responses with reconciliation
        let mut combined_content = String::new();
        let mut total_confidence = 0.0;

        for response in &final_responses {
            combined_content.push_str(&response.content);
            combined_content.push('\n');
            total_confidence += response.confidence;
        }

        Ok(MoaResponse {
            content: combined_content,
            confidence: total_confidence / final_responses.len() as f32,
            agent_responses: final_responses,
        })
    }

    fn name(&self) -> &str {
        "reconcile"
    }
}

// Helper functions for conflict resolution
fn matches_conflict_type(conflict: &Conflict, condition: &str) -> bool {
    match (conflict.conflict_type, condition) {
        (ConflictType::Factual, "factual_contradiction") => true,
        (ConflictType::Logical, "logical_inconsistency") => true,
        (ConflictType::Perspective, "perspective_difference") => true,
        (ConflictType::Incomplete, "incomplete_information") => true,
        _ => false,
    }
}

fn verify_against_kb(conflict: &Conflict, kb: &KnowledgeBase) -> (String, Vec<String>) {
    let mut evidence = Vec::new();
    for (domain, entries) in &kb.domain_knowledge {
        for entry in entries {
            if conflict.statements.iter().any(|s| s.contains(&entry.content)) {
                evidence.push(format!("Knowledge base entry from {}: {}", domain, entry.content));
            }
        }
    }
    (
        "Verified statements against knowledge base".into(),
        evidence,
    )
}

fn apply_logical_reasoning(conflict: &Conflict) -> (String, Vec<String>) {
    let evidence = conflict.statements.iter()
        .map(|s| format!("Logical analysis of: {}", s))
        .collect();
    (
        "Applied logical reasoning to resolve conflict".into(),
        evidence,
    )
}

fn use_most_recent_data(conflict: &Conflict) -> (String, Vec<String>) {
    (
        "Selected most recent data point".into(),
        vec!["temporal_analysis".into()],
    )
}

fn combine_perspectives(conflict: &Conflict) -> (String, Vec<String>) {
    let combined = conflict.statements.join(" AND ");
    (
        "Combined multiple valid perspectives".into(),
        vec![format!("Combined view: {}", combined)],
    )
}

fn request_clarification(conflict: &Conflict) -> (String, Vec<String>) {
    (
        "Requested additional information".into(),
        vec!["clarification_needed".into()],
    )
}

fn extract_numerical_claims(content: &str) -> HashMap<String, f64> {
    let mut claims = HashMap::new();
    // Basic number extraction - could be enhanced with more sophisticated NLP
    let words: Vec<&str> = content.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        if let Ok(value) = word.parse::<f64>() {
            if i > 0 {
                claims.insert(words[i-1].to_string(), value);
            }
        }
    }
    claims
} 