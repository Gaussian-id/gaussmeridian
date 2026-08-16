//! Core types and data structures

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    CostOptimized,
    SpeedOptimized,
    Provider(String),
    OpenRouter,
    LoadBalanced,
    Fallback {
        primary: String,
        fallbacks: Vec<String>,
    },
    ModelBased,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageInfo {
    pub request_id: String,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BalanceInfo {
    pub balance: f64,
    pub currency: String,
    pub last_updated: DateTime<Utc>,
}
