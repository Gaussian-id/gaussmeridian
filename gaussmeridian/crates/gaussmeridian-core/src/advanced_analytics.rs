//! Advanced Analytics Module for GaussMeridian
//!
//! Provides cost prediction, usage forecasting, and optimization recommendations.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "db")]
use gaussmeridian_db::{client::DatabaseClient, error::DatabaseError};

/// Cost prediction service
#[cfg(feature = "db")]
pub struct CostPredictor {
    db_client: DatabaseClient,
}

#[cfg(feature = "db")]
impl CostPredictor {
    pub fn new(db_client: DatabaseClient) -> Self {
        Self { db_client }
    }

    /// Predict cost for next period based on historical data
    pub async fn predict_cost(
        &self,
        user_id: &str,
        days_ahead: u32,
    ) -> Result<CostPrediction, DatabaseError> {
        // Get historical data (last 30 days)
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(30);

        let escaped_user_id = user_id.replace("'", "''");
        let query = format!(
            "SELECT \
                DATE(created_at) as date, \
                SUM(cost) as daily_cost, \
                SUM(total_tokens) as daily_tokens, \
                COUNT() as daily_requests \
             FROM requests \
             WHERE user_id = '{}' \
             AND created_at >= time::unix({}) \
             AND created_at <= time::unix({}) \
             GROUP BY DATE(created_at) \
             ORDER BY date ASC",
            escaped_user_id,
            start_date.timestamp(),
            end_date.timestamp()
        );

        let mut response = self.db_client.query(&query).await?;
        let daily_stats: Option<Vec<DailyStat>> = response.take(0)?;

        if let Some(stats) = daily_stats {
            // Calculate trend using linear regression
            let prediction = self.calculate_trend(&stats, days_ahead);
            Ok(prediction)
        } else {
            Ok(CostPrediction::default())
        }
    }

    /// Calculate optimization recommendations
    pub async fn get_optimization_recommendations(
        &self,
        user_id: &str,
    ) -> Result<Vec<Recommendation>, DatabaseError> {
        let mut recommendations = Vec::new();

        // Get model usage distribution
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(7);

        let escaped_user_id = user_id.replace("'", "''");
        let query = format!(
            "SELECT \
                model, \
                provider, \
                COUNT() as request_count, \
                SUM(cost) as total_cost, \
                AVG(cost) as avg_cost, \
                SUM(total_tokens) as total_tokens \
             FROM requests \
             WHERE user_id = '{}' \
             AND created_at >= time::unix({}) \
             GROUP BY model, provider \
             ORDER BY total_cost DESC",
            escaped_user_id,
            start_date.timestamp()
        );

        let mut response = self.db_client.query(&query).await?;
        let model_stats: Option<Vec<ModelStat>> = response.take(0)?;

        if let Some(stats) = model_stats {
            // Analyze for optimization opportunities
            for stat in stats {
                // Check if using expensive models for simple tasks
                if stat.model.contains("gpt-4") && stat.avg_cost < 0.01 {
                    recommendations.push(Recommendation {
                        priority: RecommendationPriority::High,
                        category: RecommendationCategory::CostOptimization,
                        title: format!("Consider using GPT-3.5 instead of {}", stat.model),
                        description: format!(
                            "Your average cost per request is ${:.4}, suggesting simple tasks. \
                             GPT-3.5 could save ~90% cost with similar quality.",
                            stat.avg_cost
                        ),
                        estimated_savings: stat.total_cost * 0.9,
                        impact: Impact::High,
                    });
                }

                // Check for high-volume models
                if stat.request_count > 1000 {
                    recommendations.push(Recommendation {
                        priority: RecommendationPriority::Medium,
                        category: RecommendationCategory::Caching,
                        title: format!("Enable caching for {}", stat.model),
                        description: format!(
                            "You made {} requests to {}. Enabling semantic caching could \
                             reduce costs by 30-50% for repeated queries.",
                            stat.request_count, stat.model
                        ),
                        estimated_savings: stat.total_cost * 0.4,
                        impact: Impact::Medium,
                    });
                }
            }
        }

        Ok(recommendations)
    }

    fn calculate_trend(&self, stats: &[DailyStat], days_ahead: u32) -> CostPrediction {
        if stats.is_empty() {
            return CostPrediction::default();
        }

        // Simple linear regression
        let n = stats.len() as f64;
        let sum_x: f64 = (0..stats.len()).map(|i| i as f64).sum();
        let sum_y: f64 = stats.iter().map(|s| s.daily_cost).sum();
        let sum_xy: f64 = stats
            .iter()
            .enumerate()
            .map(|(i, s)| i as f64 * s.daily_cost)
            .sum();
        let sum_x2: f64 = (0..stats.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        let intercept = (sum_y - slope * sum_x) / n;

        // Predict future values
        let mut daily_predictions = Vec::new();
        let last_x = stats.len() as f64;

        for day in 1..=days_ahead {
            let x = last_x + day as f64;
            let predicted_cost = slope * x + intercept;
            daily_predictions.push(predicted_cost.max(0.0)); // Can't be negative
        }

        let predicted_total: f64 = daily_predictions.iter().sum();

        // Calculate confidence based on data variance
        let avg_cost = sum_y / n;
        let variance: f64 = stats
            .iter()
            .map(|s| (s.daily_cost - avg_cost).powi(2))
            .sum::<f64>()
            / n;
        let confidence = 1.0 / (1.0 + variance / avg_cost.powi(2));

        CostPrediction {
            predicted_cost: predicted_total,
            confidence: confidence.min(1.0).max(0.0),
            daily_breakdown: daily_predictions,
            trend: if slope > 0.0 {
                Trend::Increasing
            } else if slope < 0.0 {
                Trend::Decreasing
            } else {
                Trend::Stable
            },
            historical_average: avg_cost,
        }
    }
}

/// Usage forecasting service
#[cfg(feature = "db")]
pub struct UsageForecaster {
    db_client: DatabaseClient,
}

#[cfg(feature = "db")]
impl UsageForecaster {
    pub fn new(db_client: DatabaseClient) -> Self {
        Self { db_client }
    }

    /// Forecast usage for next period
    pub async fn forecast_usage(
        &self,
        user_id: &str,
        days_ahead: u32,
    ) -> Result<UsageForecast, DatabaseError> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(30);

        let escaped_user_id = user_id.replace("'", "''");
        let query = format!(
            "SELECT \
                DATE(created_at) as date, \
                COUNT() as request_count, \
                SUM(total_tokens) as token_count, \
                SUM(prompt_tokens) as prompt_tokens, \
                SUM(completion_tokens) as completion_tokens \
             FROM requests \
             WHERE user_id = '{}' \
             AND created_at >= time::unix({}) \
             GROUP BY DATE(created_at) \
             ORDER BY date ASC",
            escaped_user_id,
            start_date.timestamp()
        );

        let mut response = self.db_client.query(&query).await?;
        let daily_usage: Option<Vec<DailyUsage>> = response.take(0)?;

        if let Some(usage) = daily_usage {
            Ok(self.calculate_usage_trend(&usage, days_ahead))
        } else {
            Ok(UsageForecast::default())
        }
    }

    fn calculate_usage_trend(&self, usage: &[DailyUsage], days_ahead: u32) -> UsageForecast {
        if usage.is_empty() {
            return UsageForecast::default();
        }

        let avg_requests: u64 = usage.iter().map(|u| u.request_count).sum::<u64>()
            / usage.len() as u64;
        let avg_tokens: u64 =
            usage.iter().map(|u| u.token_count).sum::<u64>() / usage.len() as u64;

        UsageForecast {
            predicted_requests: avg_requests * days_ahead as u64,
            predicted_tokens: avg_tokens * days_ahead as u64,
            daily_average_requests: avg_requests,
            daily_average_tokens: avg_tokens,
            confidence: 0.8, // Fixed confidence for simple average
        }
    }
}

/// A/B testing framework
pub struct ABTestFramework {
    experiments: HashMap<String, Experiment>,
}

impl ABTestFramework {
    pub fn new() -> Self {
        Self {
            experiments: HashMap::new(),
        }
    }

    /// Create a new A/B test
    pub fn create_experiment(&mut self, experiment: Experiment) {
        self.experiments
            .insert(experiment.id.clone(), experiment);
    }

    /// Get variant for a user
    pub fn get_variant(&self, experiment_id: &str, user_id: &str) -> Option<String> {
        self.experiments.get(experiment_id).and_then(|exp| {
            // Simple hash-based assignment
            let hash = self.hash_user(user_id);
            let variant_index = (hash % exp.variants.len() as u64) as usize;
            Some(exp.variants[variant_index].clone())
        })
    }

    /// Record experiment result
    pub fn record_result(&mut self, experiment_id: &str, variant: &str, metric: f64) {
        if let Some(exp) = self.experiments.get_mut(experiment_id) {
            exp.results
                .entry(variant.to_string())
                .or_insert_with(Vec::new)
                .push(metric);
        }
    }

    /// Get experiment results
    pub fn get_results(&self, experiment_id: &str) -> Option<ExperimentResults> {
        self.experiments.get(experiment_id).map(|exp| {
            let variant_stats: HashMap<String, VariantStats> = exp
                .variants
                .iter()
                .map(|v| {
                    let results = exp.results.get(v).cloned().unwrap_or_default();
                    let stats = self.calculate_stats(&results);
                    (v.clone(), stats)
                })
                .collect();

            ExperimentResults {
                experiment_id: exp.id.clone(),
                winner: self.determine_winner(&variant_stats),
                variant_stats,
            }
        })
    }

    fn hash_user(&self, user_id: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        hasher.finish()
    }

    fn calculate_stats(&self, values: &[f64]) -> VariantStats {
        if values.is_empty() {
            return VariantStats::default();
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance =
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

        VariantStats {
            sample_size: values.len(),
            mean,
            std_dev: variance.sqrt(),
            min: values.iter().cloned().fold(f64::INFINITY, f64::min),
            max: values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        }
    }

    fn determine_winner(&self, stats: &HashMap<String, VariantStats>) -> Option<String> {
        stats
            .iter()
            .max_by(|(_, a), (_, b)| a.mean.partial_cmp(&b.mean).unwrap())
            .map(|(k, _)| k.clone())
    }
}

impl Default for ABTestFramework {
    fn default() -> Self {
        Self::new()
    }
}

// Types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStat {
    pub date: String,
    pub daily_cost: f64,
    pub daily_tokens: u64,
    pub daily_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStat {
    pub model: String,
    pub provider: String,
    pub request_count: u64,
    pub total_cost: f64,
    pub avg_cost: f64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: String,
    pub request_count: u64,
    pub token_count: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostPrediction {
    pub predicted_cost: f64,
    pub confidence: f64,
    pub daily_breakdown: Vec<f64>,
    pub trend: Trend,
    pub historical_average: f64,
}

impl Default for CostPrediction {
    fn default() -> Self {
        Self {
            predicted_cost: 0.0,
            confidence: 0.0,
            daily_breakdown: Vec::new(),
            trend: Trend::Stable,
            historical_average: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trend {
    Increasing,
    Decreasing,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub priority: RecommendationPriority,
    pub category: RecommendationCategory,
    pub title: String,
    pub description: String,
    pub estimated_savings: f64,
    pub impact: Impact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationCategory {
    CostOptimization,
    Performance,
    Caching,
    ModelSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Impact {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageForecast {
    pub predicted_requests: u64,
    pub predicted_tokens: u64,
    pub daily_average_requests: u64,
    pub daily_average_tokens: u64,
    pub confidence: f64,
}

impl Default for UsageForecast {
    fn default() -> Self {
        Self {
            predicted_requests: 0,
            predicted_tokens: 0,
            daily_average_requests: 0,
            daily_average_tokens: 0,
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Experiment {
    pub id: String,
    pub name: String,
    pub variants: Vec<String>,
    pub results: HashMap<String, Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    pub experiment_id: String,
    pub variant_stats: HashMap<String, VariantStats>,
    pub winner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VariantStats {
    pub sample_size: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ab_testing() {
        let mut framework = ABTestFramework::new();

        let experiment = Experiment {
            id: "test_1".to_string(),
            name: "Model comparison".to_string(),
            variants: vec!["gpt-4".to_string(), "gpt-3.5".to_string()],
            results: HashMap::new(),
        };

        framework.create_experiment(experiment);

        // Assign variants
        let variant1 = framework.get_variant("test_1", "user1");
        let variant2 = framework.get_variant("test_1", "user1"); // Same user
        assert_eq!(variant1, variant2); // Should be consistent

        // Record results
        framework.record_result("test_1", "gpt-4", 0.05);
        framework.record_result("test_1", "gpt-3.5", 0.02);

        // Get results
        let results = framework.get_results("test_1");
        assert!(results.is_some());
    }
}

