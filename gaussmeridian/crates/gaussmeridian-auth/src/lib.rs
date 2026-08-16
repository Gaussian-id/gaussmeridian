//! Authentication and authorization for GaussMeridian
//!
//! This crate provides comprehensive authentication and authorization including:
//! - API key management
//! - JWT token handling
//! - OAuth2 integration
//! - Rate limiting
//! - Multi-tenant support
//! - Role-based access control

pub mod api_key;
pub mod auth_manager;
pub mod byok;
pub mod error;
pub mod jwt;
pub mod oauth2;
pub mod rate_limit;
pub mod redis_rate_limit;
pub mod rbac;
pub mod token_revocation;

pub use api_key::{ApiKeyData, ApiKeyManager};
pub use auth_manager::{AuthManager, LoginCredentials, UserCredentials};
#[cfg(feature = "db")]
pub use auth_manager::{RefreshError, RefreshRotation};
pub use byok::{ByokError, ByokVault};
pub use error::AuthError;
pub use jwt::JwtManager;
pub use oauth2::{OAuth2Config, OAuth2Manager, OAuth2Provider, OAuth2Token};
pub use rate_limit::{RateLimit, RateLimiter};
pub use redis_rate_limit::RedisRateLimiter;
pub use rbac::{
    AccessContext, Permission, PolicyCondition, PolicyRule, RBACManager, Resource, Role,
    StandardRole,
};
pub use token_revocation::{hash_token, TokenRevocationList};

/// Authentication mechanism that produced an [`AuthContext`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    #[default]
    ApiKey,
    Jwt,
}

/// Authentication context
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthContext {
    pub api_key: String,
    #[serde(default)]
    pub credential_kind: CredentialKind,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
    pub permissions: Vec<String>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}
