//! # GaussMOA: Advanced Mixture-of-Agents Engine
//!
//! GaussMOA is a powerful and flexible Mixture-of-Agents (MOA) engine in Rust,
//! designed for building sophisticated AI systems that leverage multiple specialized
//! agents for enhanced performance, reasoning, and reliability.
//!
//! ## Core Concepts
//!
//! - **Agents**: Independent (often LLM-based) entities with specific roles or skills.
//! - **Strategies**: Mechanisms to coordinate agents and combine their outputs.
//! - **MoA Engine**: Orchestrates the flow of requests, agent execution, and response aggregation.
//!
//! ## Key Features
//!
//! - **Flexible Agent System**: Supports various agent types (LLM, rule-based, custom).
//! - **Multiple MOA Strategies**: Includes Self-MOA, and provides a base for others like Standard, Sparse, Collaborative, etc.
//! - **Configuration**: TOML-based configuration for engine, agents, and strategies.
//! - **Extensibility**: Designed to be easily extended with new agents and strategies.
//! - **Asynchronous**: Built with Tokio for efficient concurrent processing.
//! - **Metrics & Logging**: (Planned) Comprehensive observability.
//! - **Persistent Storage**: (Planned) For caching, agent state, or conversation history.
//!
//! ## Getting Started
//!
//! Add GaussMOA to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! gaussmoa = { git = "https://github.com/your-username/gaussmoa.git" } # Or path, or crates.io version
//! ```
//!
//! ### Example Usage
//!
//! ```no_run
//! use gaussmoa::{MoaEngine, MoaConfig, MoaRequest};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a basic config.toml or load from a specific path
//!     // For this example, assume a minimal config.toml exists or MoaEngine::new can handle its absence
//!     // by using default values or requiring explicit agent addition.
//!     let config_path = "config.toml"; // Ensure this file exists and is configured
//!
//!     // Initialize engine
//!     let engine = MoaEngine::new(config_path).await?;
//!
//!     // Process a query
//!     // Note: Depending on MoaEngine::new implementation, agents might need to be added manually
//!     // if not defined in config.toml or if the default behavior doesn't suit.
//!     let response = engine.process_query(
//!         "What is the capital of France?",
//!         None // Optional context
//!     ).await?;
//!
//!     println!("Response: {}", response.content);
//!     println!("Confidence: {}", response.confidence);
//!
//!     Ok(())
//! }
//! ```
//!
//! For detailed setup, configuration, and advanced usage, please refer to the [USER_GUIDE.md](USER_GUIDE.md).
//!
//! ## Project Structure
//!
//! - `src/agents/`: Agent implementations and core Agent trait.
//! - `src/config/`: Configuration structures and loading logic.
//! - `src/error/`: Custom error types for the crate.
//! - `src/models/`: Data structures for requests, responses, etc.
//! - `src/security/`: Key management and cryptographic utilities.
//! - `src/storage/`: Data persistence logic.
//! - `src/strategies/`: MOA strategy implementations.
//! - `src/lib.rs`: Crate root and MoaEngine definition.
//! - `examples/`: Practical usage examples.
//!
//! ## Contributing
//!
//! Contributions are welcome! Please see `CONTRIBUTING.md` (to be created) for guidelines.
//!
//! ## License
//!
//! This project is licensed under the MIT License. See `LICENSE` (to be created) for details.

pub mod agents;
pub mod cache;
pub mod config;
pub mod error;
pub mod metrics;
pub mod models;
pub mod providers;
pub mod security;
pub mod storage;
pub mod strategies;
pub mod utils;

pub use crate::config::{AgentConfig, MoaConfig, MoaStrategy as MoaStrategyEnum};
pub use crate::error::{MoaError, MoaResult};
pub use crate::models::{AgentResponse, MoaRequest, MoaResponse};
pub use crate::providers::{ChatProvider, HttpChatProvider};

use crate::agents::Agent;
use chrono::Utc;
use futures::{stream::FuturesUnordered, StreamExt};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::{
    sync::{RwLock, Semaphore},
    time::timeout,
};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Main MoA engine that orchestrates all components.
pub struct MoaEngine {
    config: MoaConfig,
    agents: Vec<Arc<dyn Agent>>,
}

impl MoaEngine {
    /// Create a new MoA engine from a config file (standalone/debug mode). Uses the built-in
    /// OpenAI-compatible provider from the environment. The gateway uses [`MoaEngine::from_parts`].
    pub async fn new(config_path: impl Into<PathBuf>) -> MoaResult<Self> {
        let config = crate::config::load_config(config_path.into())
            .map_err(|e| MoaError::config(e.to_string(), Some(e)))?;
        Self::from_parts(
            config,
            Arc::new(crate::providers::HttpChatProvider::from_env()),
        )
        .await
    }

    /// Build the engine **in-process** from an explicit config plus an injected chat provider
    /// (Seam 2 + Seam 3) — no config file, no key file, no sidecar process. The GaussMeridian
    /// gateway constructs the engine this way, supplying its shared `gaussmeridian-providers` stack,
    /// so MoA agents authenticate/bill through the same path as normal requests.
    pub async fn from_parts(
        config: MoaConfig,
        provider: Arc<dyn crate::providers::ChatProvider>,
    ) -> MoaResult<Self> {
        let mut agents_list: Vec<Arc<dyn Agent>> = Vec::new();
        for agent_conf in &config.agents {
            let agent_instance = crate::agents::create_agent(agent_conf, provider.clone())?;
            agents_list.push(Arc::from(agent_instance));
        }
        info!("Initialized MoA engine with {} agents", agents_list.len());
        Ok(Self {
            config,
            agents: agents_list,
        })
    }

    /// Add an agent to the mixture
    pub async fn add_agent(&mut self, agent: Box<dyn Agent>) -> MoaResult<()> {
        if self.agents.iter().any(|a| a.get_id() == agent.get_id()) {
            return Err(MoaError::agent(
                "already exists".to_string(),
                agent.get_id().to_string(),
                None::<Box<dyn std::error::Error + Send + Sync>>,
            ));
        }
        self.agents.push(Arc::from(agent));
        Ok(())
    }

    /// Remove an agent from the mixture
    pub async fn remove_agent(&mut self, name: &str) -> MoaResult<()> {
        let initial_len = self.agents.len();
        self.agents.retain(|agent| agent.get_name() != name);
        if self.agents.len() == initial_len {
            return Err(MoaError::not_found(
                format!("Agent '{}' not found for removal", name),
                None::<String>,
            ));
        }
        Ok(())
    }

    /// List all configured agents
    pub async fn list_agents(&self) -> Vec<(String, crate::config::AgentRole, f32)> {
        self.agents
            .iter()
            .map(|agent| {
                let metrics = agent.get_metrics();
                (
                    agent.get_id().to_string(),
                    agent.get_config().role.clone(),
                    metrics.success_rate,
                )
            })
            .collect()
    }

    /// Get metrics for an agent
    pub async fn get_agent_metrics(&self, name: &str) -> Option<crate::agents::AgentMetrics> {
        self.agents
            .iter()
            .find(|a| a.get_name() == name)
            .map(|agent| agent.get_metrics())
    }

    /// Returns true if no agents are configured
    pub async fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Process a query through the MoA pipeline
    pub async fn process_query(
        &self,
        query: &str,
        context: Option<&str>,
    ) -> MoaResult<MoaResponse> {
        if self.is_empty().await {
            return Err(MoaError::config(
                "No agents configured".to_string(),
                None::<Box<dyn std::error::Error + Send + Sync>>,
            ));
        }

        let request = MoaRequest::new(query.to_string(), context.map(String::from));

        // SelfMoa strategy: one agent sampled N times (its own aggregation).
        if let MoaStrategyEnum::SelfMoa(ref self_moa_config) = self.config.strategy {
            let first = self.agents.first().ok_or_else(|| {
                MoaError::agent(
                    "No agents available for SelfMoa".to_string(),
                    "MoaEngine".to_string(),
                    None::<Box<dyn std::error::Error + Send + Sync>>,
                )
            })?;
            let self_moa_agent =
                crate::agents::self_moa::SelfMoaAgent::new(first.clone(), self_moa_config.samples);
            let r = self_moa_agent.process_request(&request).await?;
            return Ok(MoaResponse {
                id: Uuid::new_v4(),
                content: r.content.clone(),
                confidence: r.confidence as f32,
                agent_responses: vec![r],
                timestamp: Utc::now(),
                metrics: Default::default(),
            });
        }

        // Default MoA: fan out to ALL agents in PARALLEL, isolate per-agent failures (a slow or
        // erroring agent never fails the run), then aggregate best-of-N by confidence. This is the
        // actual mixture — previously the engine only ever called `agents.first()`.
        let mut inflight = FuturesUnordered::new();
        for agent in &self.agents {
            let agent = agent.clone();
            let req = request.clone();
            inflight.push(async move { agent.process_request(&req).await });
        }
        let mut agent_responses: Vec<AgentResponse> = Vec::new();
        while let Some(result) = inflight.next().await {
            match result {
                Ok(r) => agent_responses.push(r),
                Err(e) => warn!("MoA agent failed (continuing with the rest): {}", e),
            }
        }
        if agent_responses.is_empty() {
            return Err(MoaError::agent(
                "all MoA agents failed".to_string(),
                "MoaEngine".to_string(),
                None::<Box<dyn std::error::Error + Send + Sync>>,
            ));
        }

        let best = agent_responses
            .iter()
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("non-empty")
            .clone();
        let avg_confidence = (agent_responses.iter().map(|r| r.confidence).sum::<f64>()
            / agent_responses.len() as f64) as f32;

        Ok(MoaResponse {
            id: Uuid::new_v4(),
            content: best.content,
            confidence: avg_confidence,
            agent_responses,
            timestamp: Utc::now(),
            metrics: Default::default(),
        })
    }
}

impl Drop for MoaEngine {
    fn drop(&mut self) {
        info!("Shutting down MoA engine");
    }
}

/// Resource management constants
const MAX_CONCURRENT_REQUESTS: usize = 100;
const MAX_BATCH_SIZE: usize = 10;
const CACHE_TTL_SECS: u64 = 3600;
const CLEANUP_INTERVAL_SECS: u64 = 300; // 5 minutes

/// Resource pool for managing shared resources
pub struct ResourcePool {
    /// Semaphore for limiting concurrent requests
    request_limiter: Arc<Semaphore>,
    /// Cache for response data
    response_cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Batch processor for requests
    batch_processor: Arc<RwLock<BatchProcessor>>,
    /// Cleanup task handle
    _cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

/// Cache entry with TTL
#[derive(Clone)]
struct CacheEntry {
    data: Vec<u8>,
    expires_at: Instant,
}

/// Batch processor for requests
struct BatchProcessor {
    pending_requests: Vec<PendingRequest>,
    last_processed: Instant,
}

struct PendingRequest {
    id: String,
    data: Vec<u8>,
}

impl ResourcePool {
    pub fn new() -> Self {
        let response_cache = Arc::new(RwLock::new(HashMap::new()));
        let cache_clone = Arc::clone(&response_cache);

        // Spawn cleanup task
        let cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
            loop {
                interval.tick().await;
                Self::cleanup_expired_cache_entries(&cache_clone).await;
            }
        });

        Self {
            request_limiter: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
            response_cache,
            batch_processor: Arc::new(RwLock::new(BatchProcessor {
                pending_requests: Vec::new(),
                last_processed: Instant::now(),
            })),
            _cleanup_task: Some(cleanup_task),
        }
    }

    /// Acquire a permit for processing with timeout
    pub async fn acquire_permit(&self) -> MoaResult<tokio::sync::SemaphorePermit<'_>> {
        match timeout(Duration::from_secs(10), self.request_limiter.acquire()).await {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(e)) => Err(MoaError::resource(
                format!("Failed to acquire permit: {}", e),
                "SemaphorePermit".to_string(),
                Some(Box::new(e)),
            )),
            Err(_) => Err(MoaError::timeout(
                "Timeout acquiring permit".to_string(),
                Duration::from_secs(10),
            )),
        }
    }

    /// Get cached response if available and not expired
    pub async fn get_cached(&self, key: &str) -> Option<Vec<u8>> {
        let cache = self.response_cache.read().await;
        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// Cache response data with TTL
    pub async fn cache_response(&self, key: String, data: Vec<u8>) {
        let mut cache = self.response_cache.write().await;
        cache.insert(
            key,
            CacheEntry {
                data,
                expires_at: Instant::now() + Duration::from_secs(CACHE_TTL_SECS),
            },
        );
    }

    /// Add request to batch with improved error handling
    pub async fn add_to_batch(&self, id: String, data: Vec<u8>) -> MoaResult<()> {
        let mut processor = self.batch_processor.write().await;
        processor.pending_requests.push(PendingRequest { id, data });

        // Process batch if full or old enough
        if processor.pending_requests.len() >= MAX_BATCH_SIZE
            || processor.last_processed.elapsed() > Duration::from_secs(1)
        {
            drop(processor); // Release lock before processing
            self.process_batch().await?;
        }
        Ok(())
    }

    /// Process pending batch with improved error handling and timeout
    async fn process_batch(&self) -> MoaResult<()> {
        let mut guard = self.batch_processor.write().await;
        if guard.pending_requests.is_empty()
            || guard.last_processed.elapsed() < Duration::from_secs(5)
        {
            // Example batching window
            return Ok(());
        }

        let requests_to_process = std::mem::take(&mut guard.pending_requests);
        guard.last_processed = Instant::now();
        drop(guard); // Release lock before long-running operations

        if !requests_to_process.is_empty() {
            info!("Processing batch of {} requests", requests_to_process.len());
            let mut tasks = FuturesUnordered::new();
            let mut errors: Vec<MoaError> = Vec::new(); // To collect errors

            for request in requests_to_process {
                let request_id = request.id.clone();
                let req_clone = request; // request is moved into the async block

                tasks.push(async move {
                    match timeout(
                        Duration::from_secs(30),
                        process_request(req_clone), // req_clone is moved here
                    )
                    .await
                    {
                        Ok(Ok(result)) => Ok((request_id, result)),
                        Ok(Err(e)) => {
                            // Log the error here before it's moved if needed, or clone it.
                            // For now, let's assume e will be moved into the errors vector and logged later.
                            Err(e)
                        }
                        Err(_) => Err(MoaError::timeout(
                            format!("Timeout processing request {}", request_id),
                            Duration::from_secs(30),
                        )),
                    }
                });
            }

            while let Some(result) = tasks.next().await {
                if let Err(e) = result {
                    // Log the error here by reference as it's about to be moved into the `errors` vector.
                    error!("Batch processing error: {}", &e);
                    errors.push(e); // e is moved here
                }
            }

            // Example: if you need to handle collected errors specifically
            if !errors.is_empty() {
                // This is just an example; actual error handling might differ.
                // For instance, you might return the first error, or a combined error.
                // The original code implied logging each error within the loop, which is fine.
                // The previous attempt to log after a loop `for e in errors { error!("{}", &e); }` is also fine.
                // The key is not to use `e` after it has been moved into the `errors` vector.
            }
        }
        Ok(())
    }

    /// Cleanup expired cache entries
    async fn cleanup_expired_cache_entries(cache: &Arc<RwLock<HashMap<String, CacheEntry>>>) {
        let mut cache = cache.write().await;
        let now = Instant::now();
        cache.retain(|_, entry| entry.expires_at > now);
    }
}

impl Default for ResourcePool {
    fn default() -> Self {
        Self::new()
    }
}

/// Process a single request with improved error handling
async fn process_request(request: PendingRequest) -> MoaResult<Vec<u8>> {
    let request_id = request.id.clone();
    match timeout(Duration::from_secs(60), process_inner_request(request)).await {
        Ok(result) => result,
        Err(_) => Err(MoaError::timeout(
            format!("Request {} expired before processing", request_id),
            Duration::from_secs(60),
        )),
    }
}

async fn process_inner_request(request: PendingRequest) -> MoaResult<Vec<u8>> {
    // Actual processing logic would go here
    // For now, just return the data
    Ok(request.data)
}

/// Cleanup handler for resource pool
impl Drop for ResourcePool {
    fn drop(&mut self) {
        if let Some(task) = self._cleanup_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    // Sleeps CACHE_TTL_SECS + 1 (~1 hour) to observe cache expiry — quarantined so
    // `cargo test --workspace` doesn't hang. Run explicitly with `-- --ignored` if needed.
    #[ignore = "sleeps ~1h for TTL expiry — run with --ignored"]
    #[tokio::test]
    async fn test_resource_pool() {
        let pool = ResourcePool::new();

        // Test permit acquisition
        let permit = pool.acquire_permit().await.unwrap();
        drop(permit);

        // Test caching
        let key = "test_key".to_string();
        let data = vec![1, 2, 3];
        pool.cache_response(key.clone(), data.clone()).await;

        let cached = pool.get_cached(&key).await.unwrap();
        assert_eq!(cached, data);

        // Test batch processing
        for i in 0..5 {
            pool.add_to_batch(format!("req_{}", i), vec![i as u8])
                .await
                .unwrap();
        }

        // Wait for batch processing
        sleep(Duration::from_secs(2)).await;

        // Test cache expiration
        sleep(Duration::from_secs(CACHE_TTL_SECS + 1)).await;
        assert!(pool.get_cached(&key).await.is_none());
    }
}

#[cfg(test)]
mod moa_engine_tests {
    use super::*;
    use crate::agents::{LlmAgentConfig, LlmProvider};
    use crate::config::{AgentConfig, AgentRole, AgentType};
    use crate::providers::ChatProvider;

    /// Deterministic, no-network provider (Seam 2). Proves MoA agents run through an INJECTED
    /// provider rather than the old hardcoded `DUMMY_API_KEY` HTTP client.
    #[derive(Debug)]
    struct MockProvider;

    #[async_trait::async_trait]
    impl ChatProvider for MockProvider {
        async fn complete(
            &self,
            model: &str,
            _prompt: &str,
            _temperature: f32,
            _max_tokens: usize,
        ) -> MoaResult<String> {
            Ok(format!("mock[{model}]"))
        }
    }

    fn llm_agent(name: &str, model: &str) -> AgentConfig {
        let llm = LlmAgentConfig {
            provider: LlmProvider::OpenAI {
                model: model.to_string(),
                temperature: 0.7,
                max_tokens: 32,
            },
            system_prompt: None,
            response_format: None,
            timeout_secs: 30,
            retries: None,
        };
        AgentConfig {
            name: name.to_string(),
            agent_type: AgentType::LLM,
            role: AgentRole::Primary,
            capabilities: vec![],
            config: serde_json::to_value(&llm).unwrap(),
            max_retries: 1,
            timeout_secs: 30,
        }
    }

    #[tokio::test]
    async fn fan_out_runs_all_agents_through_the_injected_provider() {
        let config = MoaConfig {
            agents: vec![llm_agent("a1", "gpt-4o-mini"), llm_agent("a2", "gpt-4o")],
            ..MoaConfig::default()
        }; // Standard strategy → the parallel fan-out path

        let engine = MoaEngine::from_parts(config, Arc::new(MockProvider))
            .await
            .unwrap();
        let resp = engine.process_query("hello", None).await.unwrap();

        // Real mixture: BOTH agents responded (the old engine only ever called agents.first()).
        assert_eq!(resp.agent_responses.len(), 2);
        assert!(resp.content.starts_with("mock["));
        let models: std::collections::HashSet<_> = resp
            .agent_responses
            .iter()
            .map(|r| r.content.clone())
            .collect();
        assert!(models.contains("mock[gpt-4o-mini]") && models.contains("mock[gpt-4o]"));
    }

    #[tokio::test]
    async fn empty_engine_errors_rather_than_panicking() {
        let engine = MoaEngine::from_parts(MoaConfig::default(), Arc::new(MockProvider))
            .await
            .unwrap();
        assert!(engine.process_query("hi", None).await.is_err());
    }
}
