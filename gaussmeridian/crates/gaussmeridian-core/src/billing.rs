//! Comprehensive billing system with detailed token cost tracking
//! 
//! This module provides OpenRouter-style billing capabilities including:
//! - Per-request cost calculation
//! - Token usage tracking (prompt, completion, total)
//! - Model pricing database
//! - Cost analytics and reporting
//! - Budget management and alerts
//! - Provider-specific pricing

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[cfg(feature = "db")]
use gaussmeridian_db::{
    client::DatabaseClient,
    error::DatabaseError,
};

/// Model pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model: String,
    pub provider: String,
    pub input_cost_per_1m_tokens: f64,  // Cost per 1 million input tokens
    pub output_cost_per_1m_tokens: f64, // Cost per 1 million output tokens
    pub currency: String,
    pub context_window: Option<u32>,
    pub effective_date: DateTime<Utc>,
}

/// Detailed cost breakdown for a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub request_id: String,
    pub model: String,
    pub provider: String,
    
    // Token usage
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cached_tokens: u32,  // Tokens served from cache
    pub total_tokens: u32,
    
    // Cost breakdown
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_savings: f64,  // Savings from cached responses
    pub total_cost: f64,
    pub currency: String,
    
    // Metadata
    pub timestamp: DateTime<Utc>,
    pub cached: bool,
}

/// Billing summary for a user/tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSummary {
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    
    // Request statistics
    pub total_requests: u64,
    pub cached_requests: u64,
    pub failed_requests: u64,
    
    // Token statistics
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_cached_tokens: u64,
    pub total_tokens: u64,
    
    // Cost statistics
    pub total_cost: f64,
    pub cache_savings: f64,
    pub effective_cost: f64,  // total_cost - cache_savings
    pub currency: String,
    
    // Model breakdown
    pub model_costs: HashMap<String, f64>,
    pub provider_costs: HashMap<String, f64>,
}

/// Budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub monthly_limit: f64,
    pub alert_threshold: f64,  // Percentage (e.g., 80.0 for 80%)
    pub hard_limit: bool,      // If true, reject requests when limit reached
    pub currency: String,
}

/// Budget status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub budget: Budget,
    pub current_spend: f64,
    pub remaining: f64,
    pub percentage_used: f64,
    pub alert_triggered: bool,
    pub limit_reached: bool,
}

/// Comprehensive billing manager
pub struct BillingManager {
    pricing: Arc<RwLock<HashMap<String, ModelPricing>>>,
    budgets: Arc<RwLock<HashMap<String, Budget>>>,
    #[cfg(feature = "db")]
    db_client: Option<Arc<DatabaseClient>>,
}

impl BillingManager {
    pub fn new(#[cfg(feature = "db")] db_client: Option<Arc<DatabaseClient>>) -> Self {
        let pricing = Arc::new(RwLock::new(Self::default_pricing()));
        
        Self {
            pricing,
            budgets: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "db")]
            db_client,
        }
    }
    
    /// Get default pricing for popular models
    fn default_pricing() -> HashMap<String, ModelPricing> {
        let mut pricing = HashMap::new();
        
        // OpenAI models
        pricing.insert(
            "openai:gpt-4".to_string(),
            ModelPricing {
                model: "gpt-4".to_string(),
                provider: "openai".to_string(),
                input_cost_per_1m_tokens: 30.0,
                output_cost_per_1m_tokens: 60.0,
                currency: "USD".to_string(),
                context_window: Some(8192),
                effective_date: Utc::now(),
            },
        );
        
        pricing.insert(
            "openai:gpt-4-turbo".to_string(),
            ModelPricing {
                model: "gpt-4-turbo".to_string(),
                provider: "openai".to_string(),
                input_cost_per_1m_tokens: 10.0,
                output_cost_per_1m_tokens: 30.0,
                currency: "USD".to_string(),
                context_window: Some(128000),
                effective_date: Utc::now(),
            },
        );
        
        pricing.insert(
            "openai:gpt-3.5-turbo".to_string(),
            ModelPricing {
                model: "gpt-3.5-turbo".to_string(),
                provider: "openai".to_string(),
                input_cost_per_1m_tokens: 1.5,
                output_cost_per_1m_tokens: 2.0,
                currency: "USD".to_string(),
                context_window: Some(16384),
                effective_date: Utc::now(),
            },
        );
        
        // Anthropic models
        pricing.insert(
            "anthropic:claude-3-opus-20240229".to_string(),
            ModelPricing {
                model: "claude-3-opus-20240229".to_string(),
                provider: "anthropic".to_string(),
                input_cost_per_1m_tokens: 15.0,
                output_cost_per_1m_tokens: 75.0,
                currency: "USD".to_string(),
                context_window: Some(200000),
                effective_date: Utc::now(),
            },
        );
        
        pricing.insert(
            "anthropic:claude-3-sonnet-20240229".to_string(),
            ModelPricing {
                model: "claude-3-sonnet-20240229".to_string(),
                provider: "anthropic".to_string(),
                input_cost_per_1m_tokens: 3.0,
                output_cost_per_1m_tokens: 15.0,
                currency: "USD".to_string(),
                context_window: Some(200000),
                effective_date: Utc::now(),
            },
        );
        
        pricing.insert(
            "anthropic:claude-3-haiku-20240307".to_string(),
            ModelPricing {
                model: "claude-3-haiku-20240307".to_string(),
                provider: "anthropic".to_string(),
                input_cost_per_1m_tokens: 0.25,
                output_cost_per_1m_tokens: 1.25,
                currency: "USD".to_string(),
                context_window: Some(200000),
                effective_date: Utc::now(),
            },
        );
        
        // Add more models as needed...
        
        pricing
    }
    
    /// Add or update model pricing
    pub async fn set_model_pricing(&self, pricing: ModelPricing) {
        let key = format!("{}:{}", pricing.provider, pricing.model);
        let provider = pricing.provider.clone();
        let model = pricing.model.clone();
        let mut pricing_map = self.pricing.write().await;
        pricing_map.insert(key, pricing);
        info!("Updated pricing for {}:{}", provider, model);
    }
    
    /// Get pricing for a specific model
    pub async fn get_model_pricing(&self, provider: &str, model: &str) -> Option<ModelPricing> {
        let key = format!("{}:{}", provider, model);
        let pricing_map = self.pricing.read().await;
        pricing_map.get(&key).cloned()
    }
    
    /// Calculate cost for a request
    pub async fn calculate_cost(
        &self,
        provider: &str,
        model: &str,
        prompt_tokens: u32,
        completion_tokens: u32,
        cached: bool,
    ) -> Option<CostBreakdown> {
        let pricing = self.get_model_pricing(provider, model).await?;
        
        let input_cost = (prompt_tokens as f64 / 1_000_000.0) * pricing.input_cost_per_1m_tokens;
        let output_cost = (completion_tokens as f64 / 1_000_000.0) * pricing.output_cost_per_1m_tokens;
        
        let cache_savings = if cached {
            input_cost + output_cost  // Full savings if from cache
        } else {
            0.0
        };
        
        let total_cost = if cached { 0.0 } else { input_cost + output_cost };
        
        Some(CostBreakdown {
            request_id: String::new(),  // Will be set by caller
            model: model.to_string(),
            provider: provider.to_string(),
            prompt_tokens,
            completion_tokens,
            cached_tokens: if cached { prompt_tokens + completion_tokens } else { 0 },
            total_tokens: prompt_tokens + completion_tokens,
            input_cost,
            output_cost,
            cache_savings,
            total_cost,
            currency: pricing.currency,
            timestamp: Utc::now(),
            cached,
        })
    }
    
    /// Set budget for a user/tenant
    pub async fn set_budget(&self, budget: Budget) {
        let key = if let Some(tenant_id) = &budget.tenant_id {
            format!("{}:{}", budget.user_id, tenant_id)
        } else {
            budget.user_id.clone()
        };
        
        let mut budgets = self.budgets.write().await;
        budgets.insert(key, budget);
    }
    
    /// Check budget status
    #[cfg(feature = "db")]
    pub async fn check_budget(
        &self,
        user_id: &str,
        tenant_id: Option<&str>,
    ) -> Result<BudgetStatus, DatabaseError> {
        let key = if let Some(tid) = tenant_id {
            format!("{}:{}", user_id, tid)
        } else {
            user_id.to_string()
        };
        
        let budgets = self.budgets.read().await;
        let budget = budgets.get(&key).ok_or_else(|| {
            DatabaseError::Query("Budget not found".to_string())
        })?;
        
        // Get current month's spending
        let now = Utc::now();
        let year = now.year();
        let month = now.month();
        let month_start_naive = chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let month_start_dt = DateTime::<Utc>::from_naive_utc_and_offset(
            month_start_naive.and_hms_opt(0, 0, 0).unwrap(),
            Utc
        );
        
        let current_spend = self.get_period_cost(user_id, tenant_id, month_start_dt, now).await?;
        
        let percentage_used = (current_spend / budget.monthly_limit) * 100.0;
        let alert_triggered = percentage_used >= budget.alert_threshold;
        let limit_reached = budget.hard_limit && current_spend >= budget.monthly_limit;
        
        Ok(BudgetStatus {
            budget: budget.clone(),
            current_spend,
            remaining: budget.monthly_limit - current_spend,
            percentage_used,
            alert_triggered,
            limit_reached,
        })
    }
    
    /// Get total cost for a period
    #[cfg(feature = "db")]
    async fn get_period_cost(
        &self,
        user_id: &str,
        tenant_id: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<f64, DatabaseError> {
        let db_client = self.db_client.as_ref().ok_or_else(|| {
            DatabaseError::Query("Database client not available".to_string())
        })?;
        
        let escaped_user_id = user_id.replace("'", "''");
        let mut query = format!(
            "SELECT SUM(cost) as total_cost FROM requests \
             WHERE user_id = '{}' \
             AND created_at >= time::unix({}) \
             AND created_at <= time::unix({})",
            escaped_user_id,
            start.timestamp(),
            end.timestamp()
        );
        
        if let Some(tid) = tenant_id {
            let escaped_tenant_id = tid.replace("'", "''");
            query.push_str(&format!(" AND tenant_id = '{}'", escaped_tenant_id));
        }
        
        let mut response = db_client.query(&query).await?;
        let result: Option<Vec<CostResult>> = response.take(0)?;
        
        Ok(result
            .and_then(|v| v.into_iter().next())
            .map(|r| r.total_cost.unwrap_or(0.0))
            .unwrap_or(0.0))
    }
    
    /// Get billing summary for a period
    #[cfg(feature = "db")]
    pub async fn get_billing_summary(
        &self,
        user_id: &str,
        tenant_id: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<BillingSummary, DatabaseError> {
        let db_client = self.db_client.as_ref().ok_or_else(|| {
            DatabaseError::Query("Database client not available".to_string())
        })?;
        
        let escaped_user_id = user_id.replace("'", "''");
        let mut base_query = format!(
            "FROM requests WHERE user_id = '{}' \
             AND created_at >= time::unix({}) \
             AND created_at <= time::unix({})",
            escaped_user_id,
            start.timestamp(),
            end.timestamp()
        );
        
        if let Some(tid) = tenant_id {
            let escaped_tenant_id = tid.replace("'", "''");
            base_query.push_str(&format!(" AND tenant_id = '{}'", escaped_tenant_id));
        }
        
        // Get overall statistics
        let stats_query = format!(
            "SELECT \
                COUNT() as total_requests, \
                SUM(CASE WHEN status = 'cached' THEN 1 ELSE 0 END) as cached_requests, \
                SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as failed_requests, \
                SUM(prompt_tokens) as total_prompt_tokens, \
                SUM(completion_tokens) as total_completion_tokens, \
                SUM(total_tokens) as total_tokens, \
                SUM(cost) as total_cost \
             {}",
            base_query
        );
        
        let mut response = db_client.query(&stats_query).await?;
        let stats: Option<Vec<BillingStats>> = response.take(0)?;
        let stats = stats.and_then(|v| v.into_iter().next()).unwrap_or_default();
        
        // Get model breakdown
        let model_query = format!(
            "SELECT model, SUM(cost) as cost {} GROUP BY model",
            base_query
        );
        
        let mut response = db_client.query(&model_query).await?;
        let model_costs: Option<Vec<ModelCost>> = response.take(0)?;
        let model_costs: HashMap<String, f64> = model_costs
            .unwrap_or_default()
            .into_iter()
            .map(|mc| (mc.model, mc.cost))
            .collect();
        
        // Get provider breakdown
        let provider_query = format!(
            "SELECT provider, SUM(cost) as cost {} GROUP BY provider",
            base_query
        );
        
        let mut response = db_client.query(&provider_query).await?;
        let provider_costs: Option<Vec<ProviderCost>> = response.take(0)?;
        let provider_costs: HashMap<String, f64> = provider_costs
            .unwrap_or_default()
            .into_iter()
            .map(|pc| (pc.provider, pc.cost))
            .collect();
        
        Ok(BillingSummary {
            user_id: user_id.to_string(),
            tenant_id: tenant_id.map(|s| s.to_string()),
            period_start: start,
            period_end: end,
            total_requests: stats.total_requests,
            cached_requests: stats.cached_requests,
            failed_requests: stats.failed_requests,
            total_prompt_tokens: stats.total_prompt_tokens,
            total_completion_tokens: stats.total_completion_tokens,
            total_cached_tokens: 0,  // TODO: Track separately
            total_tokens: stats.total_tokens,
            total_cost: stats.total_cost,
            cache_savings: 0.0,  // TODO: Calculate from cached requests
            effective_cost: stats.total_cost,
            currency: "USD".to_string(),
            model_costs,
            provider_costs,
        })
    }
    
    /// Get all available models with pricing
    pub async fn list_models_with_pricing(&self) -> Vec<ModelPricing> {
        let pricing_map = self.pricing.read().await;
        pricing_map.values().cloned().collect()
    }
}

// Helper structs for database queries
#[cfg(feature = "db")]
#[derive(Debug, Deserialize)]
struct CostResult {
    total_cost: Option<f64>,
}

#[cfg(feature = "db")]
#[derive(Debug, Deserialize, Default)]
struct BillingStats {
    #[serde(default)]
    total_requests: u64,
    #[serde(default)]
    cached_requests: u64,
    #[serde(default)]
    failed_requests: u64,
    #[serde(default)]
    total_prompt_tokens: u64,
    #[serde(default)]
    total_completion_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
    #[serde(default)]
    total_cost: f64,
}

#[cfg(feature = "db")]
#[derive(Debug, Deserialize)]
struct ModelCost {
    model: String,
    cost: f64,
}

#[cfg(feature = "db")]
#[derive(Debug, Deserialize)]
struct ProviderCost {
    provider: String,
    cost: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_calculation() {
        let billing_manager = BillingManager::new(#[cfg(feature = "db")] None);
        
        let cost = billing_manager
            .calculate_cost("openai", "gpt-4", 1000, 500, false)
            .await
            .unwrap();
        
        assert_eq!(cost.prompt_tokens, 1000);
        assert_eq!(cost.completion_tokens, 500);
        assert!(cost.total_cost > 0.0);
        assert!(!cost.cached);
    }

    #[tokio::test]
    async fn test_cached_request_cost() {
        let billing_manager = BillingManager::new(#[cfg(feature = "db")] None);
        
        let cost = billing_manager
            .calculate_cost("openai", "gpt-4", 1000, 500, true)
            .await
            .unwrap();
        
        assert_eq!(cost.total_cost, 0.0);
        assert!(cost.cache_savings > 0.0);
        assert!(cost.cached);
    }

    #[tokio::test]
    async fn test_pricing_update() {
        let billing_manager = BillingManager::new(#[cfg(feature = "db")] None);
        
        let custom_pricing = ModelPricing {
            model: "custom-model".to_string(),
            provider: "custom-provider".to_string(),
            input_cost_per_1m_tokens: 5.0,
            output_cost_per_1m_tokens: 10.0,
            currency: "USD".to_string(),
            context_window: Some(4096),
            effective_date: Utc::now(),
        };
        
        billing_manager.set_model_pricing(custom_pricing.clone()).await;
        
        let retrieved = billing_manager
            .get_model_pricing("custom-provider", "custom-model")
            .await
            .unwrap();
        
        assert_eq!(retrieved.model, "custom-model");
        assert_eq!(retrieved.input_cost_per_1m_tokens, 5.0);
    }
}
