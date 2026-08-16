//! Usage tracking module with SurrealDB persistence
//!
//! Provides comprehensive usage tracking for requests, responses, costs,
//! and analytics with SurrealDB as the backend.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[cfg(feature = "db")]
use gaussmeridian_db::{
    client::DatabaseClient,
    error::DatabaseError,
    request_repository::{RequestRepository, RequestRepositoryTrait},
    response_repository::{ResponseRepository, ResponseRepositoryTrait},
    schema::{Request, Response},
};

/// Usage tracking service
#[cfg(feature = "db")]
pub struct UsageTracker {
    db_client: DatabaseClient,
    request_repo: RequestRepository,
    response_repo: ResponseRepository,
}

#[cfg(feature = "db")]
impl UsageTracker {
    pub fn new(db_client: DatabaseClient) -> Self {
        let request_repo = RequestRepository::new(db_client.clone());
        let response_repo = ResponseRepository::new(db_client.clone());
        Self {
            db_client,
            request_repo,
            response_repo,
        }
    }

    /// Track a request with usage information
    pub async fn track_request(
        &self,
        request: RequestUsage,
    ) -> Result<String, DatabaseError> {
        let cost = self.calculate_cost(
            &request.model,
            &request.provider,
            request.prompt_tokens.unwrap_or(0),
            request.completion_tokens.unwrap_or(0),
        );

        let request_record = Request {
            id: None,
            request_id: request.request_id,
            user_id: request.user_id,
            api_key_id: request.api_key_id,
            tenant_id: request.tenant_id,
            model: request.model,
            provider: request.provider,
            endpoint: request.endpoint,
            prompt_tokens: request.prompt_tokens,
            completion_tokens: request.completion_tokens,
            total_tokens: request.total_tokens,
            cost: Some(cost),
            currency: Some("USD".to_string()),
            status: request.status,
            error_message: request.error_message,
            latency_ms: request.latency_ms,
            created_at: Utc::now(),
        };

        let id = self.request_repo.create(request_record).await?;
        debug!("Request usage tracked: {}", id);
        Ok(id)
    }

    /// Track a response with usage information
    pub async fn track_response(
        &self,
        response: ResponseUsage,
    ) -> Result<String, DatabaseError> {
        let cost = self.calculate_cost(
            &response.model,
            &response.provider,
            response.prompt_tokens,
            response.completion_tokens,
        );

        let response_record = Response {
            id: None,
            request_id: response.request_id,
            response_id: response.response_id,
            model: response.model,
            provider: response.provider,
            prompt_tokens: response.prompt_tokens,
            completion_tokens: response.completion_tokens,
            total_tokens: response.total_tokens,
            cost,
            currency: "USD".to_string(),
            quality_score: response.quality_score.map(|f| f as f64),
            cached: response.cached,
            created_at: Utc::now(),
        };

        let id = self.response_repo.create(response_record).await?;
        debug!("Response usage tracked: {}", id);
        Ok(id)
    }

    /// Get usage for a specific request
    pub async fn get_request_usage(
        &self,
        request_id: &str,
    ) -> Result<Option<Request>, DatabaseError> {
        self.request_repo.get_by_request_id(request_id).await
    }

    /// Get usage summary for a user
    pub async fn get_user_usage_summary(
        &self,
        user_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<UsageSummary, DatabaseError> {
        let escaped_user_id = user_id.replace("'", "''");
        let query = format!(
            "SELECT \
                COUNT() as total_requests, \
                SUM(prompt_tokens) as total_prompt_tokens, \
                SUM(completion_tokens) as total_completion_tokens, \
                SUM(total_tokens) as total_tokens, \
                SUM(cost) as total_cost \
             FROM requests \
             WHERE user_id = '{}' \
             AND created_at >= time::unix({}) \
             AND created_at <= time::unix({})",
            escaped_user_id,
            start_date.timestamp(),
            end_date.timestamp()
        );

        let mut response = self.db_client.query(&query).await?;
        let summaries: Option<Vec<UsageSummary>> = response.take(0)?;

        Ok(summaries
            .and_then(|v| v.into_iter().next())
            .unwrap_or_default())
    }

    /// Get usage summary by model
    pub async fn get_model_usage_summary(
        &self,
        user_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<ModelUsageSummary>, DatabaseError> {
        let escaped_user_id = user_id.replace("'", "''");
        let query = format!(
            "SELECT \
                model, \
                provider, \
                COUNT() as request_count, \
                SUM(total_tokens) as total_tokens, \
                SUM(cost) as total_cost \
             FROM requests \
             WHERE user_id = '{}' \
             AND created_at >= time::unix({}) \
             AND created_at <= time::unix({}) \
             GROUP BY model, provider",
            escaped_user_id,
            start_date.timestamp(),
            end_date.timestamp()
        );

        let mut response = self.db_client.query(&query).await?;
        let summaries: Option<Vec<ModelUsageSummary>> = response.take(0)?;

        Ok(summaries.unwrap_or_default())
    }

    /// Calculate cost based on model, provider, and tokens
    fn calculate_cost(
        &self,
        model: &str,
        provider: &str,
        prompt_tokens: u32,
        completion_tokens: u32,
    ) -> f64 {
        // Cost per 1K tokens (example rates, should be configurable)
        let (prompt_cost_per_1k, completion_cost_per_1k) = match (provider, model) {
            ("openai", m) if m.contains("gpt-4") => (0.03, 0.06),
            ("openai", m) if m.contains("gpt-3.5") => (0.0015, 0.002),
            ("anthropic", m) if m.contains("claude-3-opus") => (0.015, 0.075),
            ("anthropic", m) if m.contains("claude-3-sonnet") => (0.003, 0.015),
            ("anthropic", m) if m.contains("claude-3-haiku") => (0.00025, 0.00125),
            _ => (0.001, 0.002), // Default rates
        };

        let prompt_cost = (prompt_tokens as f64 / 1000.0) * prompt_cost_per_1k;
        let completion_cost = (completion_tokens as f64 / 1000.0) * completion_cost_per_1k;

        prompt_cost + completion_cost
    }

    /// Delete old usage data (cleanup job)
    pub async fn cleanup_old_data(&self, before: DateTime<Utc>) -> Result<(), DatabaseError> {
        let query = format!(
            "DELETE FROM requests WHERE created_at < time::unix({})",
            before.timestamp()
        );
        self.db_client.query(&query).await?;

        let query = format!(
            "DELETE FROM responses WHERE created_at < time::unix({})",
            before.timestamp()
        );
        self.db_client.query(&query).await?;

        info!("Cleaned up usage data before {}", before);
        Ok(())
    }
}

/// Request usage information
#[derive(Debug, Clone)]
pub struct RequestUsage {
    pub request_id: String,
    pub user_id: Option<String>,
    pub api_key_id: Option<String>,
    pub tenant_id: Option<String>,
    pub model: String,
    pub provider: String,
    pub endpoint: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub status: String,
    pub error_message: Option<String>,
    pub latency_ms: Option<u64>,
}

/// Response usage information
#[derive(Debug, Clone)]
pub struct ResponseUsage {
    pub request_id: String,
    pub response_id: String,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub quality_score: Option<f32>,
    pub cached: bool,
}

/// Usage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub total_requests: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
}

impl Default for UsageSummary {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_prompt_tokens: 0,
            total_completion_tokens: 0,
            total_tokens: 0,
            total_cost: 0.0,
        }
    }
}

/// Model usage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageSummary {
    pub model: String,
    pub provider: String,
    pub request_count: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_summary_default() {
        let summary = UsageSummary::default();
        assert_eq!(summary.total_requests, 0);
        assert_eq!(summary.total_tokens, 0);
        assert_eq!(summary.total_cost, 0.0);
    }
}

