//! Intelligent cost-aware routing strategy
//!
//! This module provides automatic routing to minimize costs while maintaining
//! quality of responses, inspired by OpenRouter's approach.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use gaussmeridian_models::{ChatCompletionRequest, Message};
use crate::provider_registry::ProviderEntry;
use crate::error::GaussMeridianError;

/// Quality tier for models
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum QualityTier {
    Budget,      // Lowest quality, lowest cost
    Standard,    // Good balance
    Premium,     // High quality
    Elite,       // Best quality, highest cost
}

/// Model capability profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    pub model: String,
    pub provider: String,
    pub quality_tier: QualityTier,
    pub quality_score: f64,  // 0.0 to 1.0
    pub cost_per_1m_tokens: f64,
    pub avg_latency_ms: f64,
    pub context_window: u32,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub supports_streaming: bool,
}

/// Request complexity assessment
#[derive(Debug, Clone, Copy)]
pub enum RequestComplexity {
    Simple,      // Basic queries, summaries
    Moderate,    // Analysis, reasoning
    Complex,     // Advanced reasoning, coding
    Expert,      // Research, complex problem solving
}

/// Cost-aware routing configuration
#[derive(Debug, Clone)]
pub struct CostAwareRoutingConfig {
    /// Enable automatic quality downgrading for simple tasks
    pub enable_auto_downgrade: bool,
    
    /// Enable automatic quality upgrade for complex tasks
    pub enable_auto_upgrade: bool,
    
    /// Maximum cost multiplier allowed (e.g., 2.0 = 2x the cheapest option)
    pub max_cost_multiplier: f64,
    
    /// Minimum quality score threshold (0.0 to 1.0)
    pub min_quality_score: f64,
    
    /// Prefer faster models when quality is similar
    pub prefer_low_latency: bool,
    
    /// Weight for cost optimization (0.0 to 1.0)
    /// 1.0 = pure cost optimization, 0.0 = pure quality optimization
    pub cost_optimization_weight: f64,
}

impl Default for CostAwareRoutingConfig {
    fn default() -> Self {
        Self {
            enable_auto_downgrade: true,
            enable_auto_upgrade: true,
            max_cost_multiplier: 5.0,
            min_quality_score: 0.6,
            prefer_low_latency: true,
            cost_optimization_weight: 0.7,  // Favor cost over quality
        }
    }
}

/// Cost-aware router that selects the optimal model
pub struct CostAwareRouter {
    config: CostAwareRoutingConfig,
    model_capabilities: Arc<RwLock<HashMap<String, ModelCapability>>>,
    routing_history: Arc<RwLock<Vec<RoutingDecision>>>,
}

/// Record of a routing decision
#[derive(Debug, Clone)]
struct RoutingDecision {
    timestamp: std::time::SystemTime,
    requested_model: String,
    selected_model: String,
    selected_provider: String,
    complexity: RequestComplexity,
    expected_cost: f64,
    reason: String,
}

impl CostAwareRouter {
    pub fn new(config: CostAwareRoutingConfig) -> Self {
        Self {
            config,
            model_capabilities: Arc::new(RwLock::new(Self::default_capabilities())),
            routing_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Initialize with default model capabilities
    fn default_capabilities() -> HashMap<String, ModelCapability> {
        let mut capabilities = HashMap::new();
        
        // OpenAI models
        capabilities.insert(
            "openai:gpt-4".to_string(),
            ModelCapability {
                model: "gpt-4".to_string(),
                provider: "openai".to_string(),
                quality_tier: QualityTier::Elite,
                quality_score: 0.95,
                cost_per_1m_tokens: 45.0,  // Average of input/output
                avg_latency_ms: 3000.0,
                context_window: 8192,
                supports_function_calling: true,
                supports_vision: false,
                supports_streaming: true,
            },
        );
        
        capabilities.insert(
            "openai:gpt-4-turbo".to_string(),
            ModelCapability {
                model: "gpt-4-turbo".to_string(),
                provider: "openai".to_string(),
                quality_tier: QualityTier::Elite,
                quality_score: 0.93,
                cost_per_1m_tokens: 20.0,
                avg_latency_ms: 2000.0,
                context_window: 128000,
                supports_function_calling: true,
                supports_vision: true,
                supports_streaming: true,
            },
        );
        
        capabilities.insert(
            "openai:gpt-3.5-turbo".to_string(),
            ModelCapability {
                model: "gpt-3.5-turbo".to_string(),
                provider: "openai".to_string(),
                quality_tier: QualityTier::Standard,
                quality_score: 0.75,
                cost_per_1m_tokens: 1.75,
                avg_latency_ms: 800.0,
                context_window: 16384,
                supports_function_calling: true,
                supports_vision: false,
                supports_streaming: true,
            },
        );
        
        // Anthropic models
        capabilities.insert(
            "anthropic:claude-3-opus-20240229".to_string(),
            ModelCapability {
                model: "claude-3-opus-20240229".to_string(),
                provider: "anthropic".to_string(),
                quality_tier: QualityTier::Elite,
                quality_score: 0.96,
                cost_per_1m_tokens: 45.0,
                avg_latency_ms: 2500.0,
                context_window: 200000,
                supports_function_calling: true,
                supports_vision: true,
                supports_streaming: true,
            },
        );
        
        capabilities.insert(
            "anthropic:claude-3-sonnet-20240229".to_string(),
            ModelCapability {
                model: "claude-3-sonnet-20240229".to_string(),
                provider: "anthropic".to_string(),
                quality_tier: QualityTier::Premium,
                quality_score: 0.88,
                cost_per_1m_tokens: 9.0,
                avg_latency_ms: 1500.0,
                context_window: 200000,
                supports_function_calling: true,
                supports_vision: true,
                supports_streaming: true,
            },
        );
        
        capabilities.insert(
            "anthropic:claude-3-haiku-20240307".to_string(),
            ModelCapability {
                model: "claude-3-haiku-20240307".to_string(),
                provider: "anthropic".to_string(),
                quality_tier: QualityTier::Standard,
                quality_score: 0.78,
                cost_per_1m_tokens: 0.75,
                avg_latency_ms: 600.0,
                context_window: 200000,
                supports_function_calling: false,
                supports_vision: true,
                supports_streaming: true,
            },
        );
        
        capabilities
    }
    
    /// Register or update model capability
    pub async fn register_model(&self, capability: ModelCapability) {
        let key = format!("{}:{}", capability.provider, capability.model);
        let mut capabilities = self.model_capabilities.write().await;
        capabilities.insert(key, capability);
    }
    
    /// Assess request complexity
    fn assess_complexity(&self, request: &ChatCompletionRequest) -> RequestComplexity {
        let message_count = request.messages.len();
        let total_length: usize = request.messages.iter()
            .map(|m| self.estimate_message_length(m))
            .sum();
        
        // Check for function calling
        let uses_functions = request.tools.is_some() || request.functions.is_some();
        
        // Check for system prompts or special instructions
        let has_system_prompt = request.messages.iter()
            .any(|m| matches!(m.role, gaussmeridian_models::Role::System));
        
        // Complexity heuristics
        if uses_functions || total_length > 2000 || message_count > 10 {
            RequestComplexity::Complex
        } else if total_length > 500 || message_count > 5 || has_system_prompt {
            RequestComplexity::Moderate
        } else {
            RequestComplexity::Simple
        }
    }
    
    fn estimate_message_length(&self, message: &Message) -> usize {
        match &message.content {
            gaussmeridian_models::Content::Text(text) => text.len(),
            gaussmeridian_models::Content::Parts(parts) => {
                parts.iter().map(|part| {
                    match part {
                        gaussmeridian_models::ContentPart::Text { text } => text.len(),
                        gaussmeridian_models::ContentPart::ImageUrl { .. } => 100, // Estimate
                    }
                }).sum()
            }
        }
    }
    
    /// Select the optimal model based on cost and quality
    pub async fn select_optimal_model(
        &self,
        request: &ChatCompletionRequest,
        available_providers: &[Arc<ProviderEntry>],
    ) -> Result<(String, String, String), GaussMeridianError> {
        let complexity = self.assess_complexity(request);
        let capabilities = self.model_capabilities.read().await;
        
        debug!("Request complexity: {:?}", complexity);
        
        // Determine required quality tier based on complexity
        let required_tier = match complexity {
            RequestComplexity::Simple => QualityTier::Budget,
            RequestComplexity::Moderate => QualityTier::Standard,
            RequestComplexity::Complex => QualityTier::Premium,
            RequestComplexity::Expert => QualityTier::Elite,
        };
        
        debug!("Required quality tier: {:?}", required_tier);
        
        // Filter available models
        let mut candidates: Vec<&ModelCapability> = capabilities.values()
            .filter(|cap| {
                // Check if provider is available
                available_providers.iter().any(|p| p.name == cap.provider)
                    && cap.quality_score >= self.config.min_quality_score
                    && (self.config.enable_auto_downgrade || cap.quality_tier >= required_tier)
            })
            .collect();
        
        if candidates.is_empty() {
            return Err(GaussMeridianError::ProviderError("No providers available".to_string()));
        }
        
        // Sort by optimization score
        candidates.sort_by(|a, b| {
            let score_a = self.calculate_optimization_score(a, required_tier);
            let score_b = self.calculate_optimization_score(b, required_tier);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        let selected = candidates[0];
        
        // Calculate estimated cost (rough estimate based on average tokens)
        let estimated_tokens = self.estimate_tokens(request);
        let estimated_cost = (estimated_tokens as f64 / 1_000_000.0) * selected.cost_per_1m_tokens;
        
        let reason = format!(
            "Selected {} (tier: {:?}, quality: {:.2}, cost: ${:.4}) for {:?} task",
            selected.model, selected.quality_tier, selected.quality_score, estimated_cost, complexity
        );
        
        info!("{}", reason);
        
        // Record decision
        let decision = RoutingDecision {
            timestamp: std::time::SystemTime::now(),
            requested_model: request.model.clone(),
            selected_model: selected.model.clone(),
            selected_provider: selected.provider.clone(),
            complexity,
            expected_cost: estimated_cost,
            reason: reason.clone(),
        };
        
        let mut history = self.routing_history.write().await;
        history.push(decision);
        
        // Keep only last 1000 decisions
        let len = history.len();
        if len > 1000 {
            history.drain(0..len - 1000);
        }
        
        Ok((
            selected.provider.clone(),
            selected.model.clone(),
            reason,
        ))
    }
    
    /// Calculate optimization score combining cost and quality
    fn calculate_optimization_score(&self, capability: &ModelCapability, required_tier: QualityTier) -> f64 {
        // Quality score (0 to 1)
        let quality_score = capability.quality_score;
        
        // Cost score (inverse normalized, 0 to 1, where 1 is cheapest)
        let max_acceptable_cost = 50.0; // $50 per million tokens
        let cost_score = 1.0 - (capability.cost_per_1m_tokens / max_acceptable_cost).min(1.0);
        
        // Latency score (0 to 1, where 1 is fastest)
        let latency_score = if self.config.prefer_low_latency {
            1.0 - (capability.avg_latency_ms / 5000.0).min(1.0)
        } else {
            1.0 // Don't factor in latency
        };
        
        // Tier bonus (prefer models at or slightly above required tier)
        let tier_bonus = if capability.quality_tier == required_tier {
            0.2
        } else if capability.quality_tier as i32 == required_tier as i32 + 1 {
            0.1
        } else {
            0.0
        };
        
        // Weighted combination
        let w_cost = self.config.cost_optimization_weight;
        let w_quality = 1.0 - w_cost;
        let w_latency = if self.config.prefer_low_latency { 0.1 } else { 0.0 };
        
        let base_score = (cost_score * w_cost) + (quality_score * w_quality) + (latency_score * w_latency);
        
        base_score + tier_bonus
    }
    
    /// Estimate tokens for request (rough approximation)
    fn estimate_tokens(&self, request: &ChatCompletionRequest) -> u32 {
        let char_count: usize = request.messages.iter()
            .map(|m| self.estimate_message_length(m))
            .sum();
        
        // Rough approximation: 4 characters per token
        let input_tokens = (char_count / 4) as u32;
        
        // Estimate output tokens based on max_tokens or default
        let output_tokens = request.max_tokens.unwrap_or(150) as u32;
        
        input_tokens + output_tokens
    }
    
    /// Get routing statistics
    pub async fn get_routing_stats(&self) -> RoutingStats {
        let history = self.routing_history.read().await;
        
        let total = history.len();
        let simple = history.iter().filter(|d| matches!(d.complexity, RequestComplexity::Simple)).count();
        let moderate = history.iter().filter(|d| matches!(d.complexity, RequestComplexity::Moderate)).count();
        let complex = history.iter().filter(|d| matches!(d.complexity, RequestComplexity::Complex)).count();
        let expert = history.iter().filter(|d| matches!(d.complexity, RequestComplexity::Expert)).count();
        
        let total_cost: f64 = history.iter().map(|d| d.expected_cost).sum();
        let avg_cost = if total > 0 { total_cost / total as f64 } else { 0.0 };
        
        RoutingStats {
            total_decisions: total,
            simple_requests: simple,
            moderate_requests: moderate,
            complex_requests: complex,
            expert_requests: expert,
            total_estimated_cost: total_cost,
            avg_cost_per_request: avg_cost,
        }
    }
}

/// Routing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingStats {
    pub total_decisions: usize,
    pub simple_requests: usize,
    pub moderate_requests: usize,
    pub complex_requests: usize,
    pub expert_requests: usize,
    pub total_estimated_cost: f64,
    pub avg_cost_per_request: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use gaussmeridian_models::{Content, Role};
    use std::collections::HashMap;

    fn create_test_request(message_count: usize, complexity: &str) -> ChatCompletionRequest {
        let messages = (0..message_count)
            .map(|i| Message {
                role: Role::User,
                content: Content::Text(complexity.repeat(i + 1)),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
                confidence: None,
            })
            .collect();
        
        ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            n: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            transforms: None,
            models: None,
            route: None,
            provider: None,
            routing_strategy: None,
            fallback_providers: None,
            cost_limit: None,
            timeout: None,
            tenant_id: None,
            request_metadata: None,
            extra: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_complexity_assessment() {
        let config = CostAwareRoutingConfig::default();
        let router = CostAwareRouter::new(config);
        
        let simple_req = create_test_request(1, "test");
        let complexity = router.assess_complexity(&simple_req);
        assert!(matches!(complexity, RequestComplexity::Simple));
        
        let complex_req = create_test_request(15, "test ");
        let complexity = router.assess_complexity(&complex_req);
        assert!(matches!(complexity, RequestComplexity::Complex));
    }

    #[tokio::test]
    async fn test_model_registration() {
        let config = CostAwareRoutingConfig::default();
        let router = CostAwareRouter::new(config);
        
        let custom_capability = ModelCapability {
            model: "custom-model".to_string(),
            provider: "custom-provider".to_string(),
            quality_tier: QualityTier::Premium,
            quality_score: 0.9,
            cost_per_1m_tokens: 10.0,
            avg_latency_ms: 1000.0,
            context_window: 8192,
            supports_function_calling: true,
            supports_vision: false,
            supports_streaming: true,
        };
        
        router.register_model(custom_capability.clone()).await;
        
        let capabilities = router.model_capabilities.read().await;
        assert!(capabilities.contains_key("custom-provider:custom-model"));
    }
}
