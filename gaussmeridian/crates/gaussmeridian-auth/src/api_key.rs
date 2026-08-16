//! API key management

use crate::rate_limit::RateLimit;
use serde::{Deserialize, Serialize};

/// API key data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyData {
    pub key_id: String,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
    pub permissions: Vec<String>,
    pub rate_limit: Option<RateLimit>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

/// API Key Manager
#[derive(Debug, Default, Clone)]
pub struct ApiKeyManager;

impl ApiKeyManager {
    /// Construct a new `ApiKeyManager`.
    ///
    /// Prefer using `ApiKeyManager::default()` when possible to follow
    /// conventional Rust patterns.
    pub fn new() -> Self {
        Self
    }
}
