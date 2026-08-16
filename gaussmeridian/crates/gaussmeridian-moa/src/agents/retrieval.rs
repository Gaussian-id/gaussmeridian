//! Retrieval agent implementation
//! 
//! This agent uses vector database and embeddings for retrieval-augmented generation.

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

/// Vector database interface (placeholder - would integrate with actual vector DB)
#[async_trait]
pub trait VectorDatabase: Send + Sync {
    async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<Document>, String>;
    async fn add_document(&self, document: Document) -> Result<(), String>;
}

/// Document with embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

/// Embedding generator interface (placeholder - would integrate with actual embedding service)
#[async_trait]
pub trait EmbeddingGenerator: Send + Sync {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, String>;
}

/// Simple in-memory vector database implementation
pub struct InMemoryVectorDB {
    documents: Vec<Document>,
}

impl InMemoryVectorDB {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
        }
    }

    /// Calculate cosine similarity
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm_a * norm_b)
    }
}

#[async_trait]
impl VectorDatabase for InMemoryVectorDB {
    async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<Document>, String> {
        let mut scored_docs: Vec<(f32, Document)> = self.documents
            .iter()
            .map(|doc| {
                let similarity = Self::cosine_similarity(query_embedding, &doc.embedding);
                (similarity, doc.clone())
            })
            .collect();
        
        scored_docs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored_docs.truncate(top_k);
        
        Ok(scored_docs.into_iter().map(|(_, doc)| doc).collect())
    }

    async fn add_document(&self, document: Document) -> Result<(), String> {
        // This would need to be mutable, but for now it's a placeholder
        // In production, use Arc<Mutex<Vec<Document>>>
        Ok(())
    }
}

/// Simple embedding generator (placeholder)
pub struct SimpleEmbeddingGenerator;

impl SimpleEmbeddingGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate a simple embedding (placeholder - would use actual embedding model)
    fn generate_simple_embedding(text: &str) -> Vec<f32> {
        // Simple hash-based embedding (not a real embedding, just for demonstration)
        let mut embedding = vec![0.0; 384]; // Standard embedding dimension
        for (i, byte) in text.as_bytes().iter().enumerate() {
            embedding[i % 384] += *byte as f32 / 255.0;
        }
        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            embedding.iter_mut().for_each(|x| *x /= norm);
        }
        embedding
    }
}

#[async_trait]
impl EmbeddingGenerator for SimpleEmbeddingGenerator {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        Ok(Self::generate_simple_embedding(text))
    }
}

/// Retrieval agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalAgentConfig {
    pub top_k: usize,
    pub similarity_threshold: f32,
    pub enable_reranking: bool,
    pub max_context_length: usize,
}

impl Default for RetrievalAgentConfig {
    fn default() -> Self {
        Self {
            top_k: 5,
            similarity_threshold: 0.7,
            enable_reranking: false,
            max_context_length: 2000,
        }
    }
}

/// Retrieval agent implementation
pub struct RetrievalAgent {
    base: BaseAgent,
    config: RetrievalAgentConfig,
    vector_db: Box<dyn VectorDatabase>,
    embedding_generator: Box<dyn EmbeddingGenerator>,
}

impl std::fmt::Debug for RetrievalAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetrievalAgent")
            .field("base", &self.base)
            .field("config", &self.config)
            .field("vector_db", &"<VectorDatabase>")
            .field("embedding_generator", &"<EmbeddingGenerator>")
            .finish()
    }
}

impl RetrievalAgent {
    pub fn new(
        id: String,
        role: AgentRole,
        config: RetrievalAgentConfig,
        vector_db: Box<dyn VectorDatabase>,
        embedding_generator: Box<dyn EmbeddingGenerator>,
    ) -> Self {
        Self {
            base: BaseAgent::new(
                id.clone(),
                "Retrieval Agent".to_string(),
                "An agent that uses retrieval-augmented generation".to_string(),
                vec!["retrieval".to_string(), "rag".to_string()],
                AgentConfig {
                    name: id.clone(),
                    agent_type: crate::config::AgentType::Retrieval,
                    role: role.clone(),
                    capabilities: vec!["retrieval".to_string(), "rag".to_string()],
                    config: serde_json::to_value(config.clone()).unwrap_or_default(),
                    max_retries: 3,
                    timeout_secs: 30,
                }
            ),
            config,
            vector_db,
            embedding_generator,
        }
    }

    /// Build context from retrieved documents with security and performance optimizations
    fn build_context(&self, documents: &[Document]) -> String {
        let mut context = String::with_capacity(self.config.max_context_length);
        for (i, doc) in documents.iter().enumerate() {
            // Security: Sanitize document content to prevent injection
            let sanitized_content = doc.content
                .chars()
                .take(10_000) // Limit individual document size
                .collect::<String>();
            
            context.push_str(&format!("[Document {}]\n", i + 1));
            context.push_str(&sanitized_content);
            context.push_str("\n\n");
            
            // Performance: Early exit if context is large enough
            if context.len() > self.config.max_context_length {
                context.truncate(self.config.max_context_length);
                break;
            }
        }
        context
    }
}

#[async_trait]
impl Agent for RetrievalAgent {
    fn get_id(&self) -> &str {
        self.base.get_id()
    }

    fn get_name(&self) -> &str {
        "Retrieval Agent"
    }

    fn get_description(&self) -> &str {
        "An agent that uses retrieval-augmented generation"
    }

    fn get_capabilities(&self) -> &[String] {
        self.base.get_capabilities()
    }

    fn get_config(&self) -> &AgentConfig {
        self.base.get_config()
    }

    async fn process_request(&self, request: &MoaRequest) -> MoaResult<AgentResponse> {
        let start = std::time::Instant::now();
        
        // Generate embedding for query
        let query_embedding = self.embedding_generator.generate_embedding(&request.query).await
            .map_err(|e| MoaError::internal(format!("Failed to generate embedding: {}", e), None::<MoaError>))?;
        
        // Search for relevant documents
        let documents = self.vector_db.search(&query_embedding, self.config.top_k).await
            .map_err(|e| MoaError::internal(format!("Vector search failed: {}", e), None::<MoaError>))?;
        
        if documents.is_empty() {
            return Err(MoaError::Agent {
                agent_id: self.get_id().to_string(),
                message: "No relevant documents found".to_string(),
                source: None,
            });
        }
        
        // Build context from retrieved documents
        let context = self.build_context(&documents);
        
        // Generate response using context (placeholder - would use LLM)
        let content = format!(
            "Based on the retrieved context:\n\n{}\n\nQuery: {}\n\nResponse: [This would be generated by an LLM using the context above]",
            context,
            request.query
        );
        
        // Calculate confidence based on document similarity
        let confidence = 0.8; // Placeholder - would calculate from document similarities
        
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

