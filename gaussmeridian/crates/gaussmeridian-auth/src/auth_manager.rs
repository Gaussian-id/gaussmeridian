//! Authentication manager that integrates with SurrealDB
//!
//! This module provides a unified authentication manager that uses
//! SurrealDB for user storage and integrates JWT and API key authentication.
//! 
//! Features:
//! - Secure Argon2id password hashing (DR-010, ratified 2026-07-16), with transparent
//!   verify + rehash-on-login support for legacy bcrypt and SHA-256 rows
//! - JWT token authentication
//! - API key authentication
//! - Role-based access control
//! - Password reset token generation
//! - User profile management

use crate::{
    api_key::ApiKeyManager,
    error::AuthError,
    jwt::JwtManager,
    rbac::{AccessContext, Permission, RBACManager, Resource},
    AuthContext,
};
#[cfg(feature = "db")]
use gaussmeridian_db::{
    api_key_repository::{ApiKeyRepository, ApiKeyRepositoryTrait},
    client::DatabaseClient,
    refresh_token_repository::{RefreshTokenRepository, RefreshTokenRepositoryTrait},
    repositories::user_repository::{UserRepository, UserRepositoryTrait},
    schema::{ApiKey, User},
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Authentication result
pub type AuthResult<T> = Result<T, AuthError>;

/// Bcrypt cost factor used only for verifying pre-DR-010 legacy hashes (and as a fallback
/// if Argon2id hashing itself fails). New passwords no longer hash with bcrypt — see
/// `hash_password`.
const BCRYPT_COST: u32 = 12;

/// User credentials for registration/login
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserCredentials {
    pub email: String,
    pub username: String,
    pub password: String,
    pub tenant_id: Option<String>,
}

/// Login credentials
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoginCredentials {
    pub email: String,
    pub password: String,
}

/// Password change request
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PasswordChangeRequest {
    pub current_password: String,
    pub new_password: String,
}

/// User profile update request
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserProfileUpdate {
    pub username: Option<String>,
    pub email: Option<String>,
}

/// Password reset token data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PasswordResetToken {
    pub token: String,
    pub user_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Outcome of a successful refresh-token rotation (PRD-25 Phase 1). Carries everything the
/// handler needs to build its `{ token, refresh_token, user }` response without re-querying.
#[cfg(feature = "db")]
pub struct RefreshRotation {
    pub user: User,
    pub access_token: String,
    /// The new RAW refresh token (64 hex) — returned to the caller exactly once.
    pub refresh_token: String,
}

/// Why a refresh-token rotation failed. Maps 1:1 to the `/v1/auth/refresh` error `code`
/// vocabulary (`token_expired` / `token_reuse_detected` / `invalid_grant`) — see the handler.
#[cfg(feature = "db")]
#[derive(Debug)]
pub enum RefreshError {
    /// The token exists but is past its `expires_at` — the family is left intact (benign).
    Expired,
    /// An already-rotated token was presented → the whole family has been revoked.
    ReuseDetected,
    /// Unknown token, inactive/missing user, or a benign lost rotation race — no family action.
    InvalidGrant,
    /// Infrastructure failure (DB error / repo not configured).
    Internal(String),
}

/// Grace window (seconds) for a benign concurrent-rotation race: a token revoked this recently
/// AND already carrying a successor is a lost single-flight race, not theft (PRD-25 §refresh-contract).
#[cfg(feature = "db")]
const REFRESH_GRACE_SECS: i64 = 10;

/// The security verdict on a refresh-token record's `{ revoked_at, replaced_by, expires_at }`
/// state, independent of any DB / user lookup.
#[cfg(feature = "db")]
#[derive(Debug, PartialEq, Eq)]
pub enum RefreshRecordDecision {
    /// Past `expires_at` (or no expiry recorded) — benign; the family is left intact.
    Expired,
    /// An already-rotated (revoked) token presented outside the grace window, or revoked with no
    /// successor recorded — treat as theft: the caller must revoke the whole family.
    ReuseDetected,
    /// Revoked within the grace window AND a successor exists — a benign concurrent-rotation race
    /// (single-flight double-fire / multi-instance): reject as invalid_grant, do NOT nuke the family.
    BenignGraceRetry,
    /// Live token (not revoked, not expired) — eligible to rotate.
    Active,
}

/// Pure classification of a refresh-token record's security state (PRD-25 Phase 1, BUG-05).
/// Extracted from `rotate_refresh_token` so every branch of the crown-jewel decision is
/// unit-testable without a DB or a mockable repository. Order is deliberate — **expiry is checked
/// first**, so a long-dead already-rotated token classifies as `Expired` (benign) rather than
/// theft. `expires_at` is `Option` to mirror the SCHEMAFULL `option<datetime>` field: a record
/// with no expiry has no lifetime and is treated as expired.
#[cfg(feature = "db")]
fn classify_refresh_record(
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    replaced_by: Option<String>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
    grace_secs: i64,
) -> RefreshRecordDecision {
    // 1. Expired (or no expiry) — benign, checked first.
    match expires_at {
        Some(exp) if exp >= now => {}
        _ => return RefreshRecordDecision::Expired,
    }

    // 2. Revocation state.
    if let Some(revoked_at) = revoked_at {
        let within_grace = (now - revoked_at).num_seconds().abs() <= grace_secs;
        if within_grace && replaced_by.is_some() {
            return RefreshRecordDecision::BenignGraceRetry;
        }
        return RefreshRecordDecision::ReuseDetected;
    }

    // 3. Live token.
    RefreshRecordDecision::Active
}

/// Authentication manager
pub struct AuthManager {
    jwt_manager: Arc<JwtManager>,
    api_key_manager: Arc<ApiKeyManager>,
    rbac_manager: Arc<RBACManager>,
    #[cfg(feature = "db")]
    user_repository: Option<Arc<UserRepository>>,
    #[cfg(feature = "db")]
    api_key_repository: Option<Arc<ApiKeyRepository>>,
    #[cfg(feature = "db")]
    refresh_token_repository: Option<Arc<RefreshTokenRepository>>,
    /// PRD-25 Phase 1 — refresh-token lifetime (seconds); default 30 d, overridable via
    /// `with_refresh_ttl` (from `SecurityConfig` in `app.rs`).
    refresh_ttl_secs: u64,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(
        jwt_manager: JwtManager,
        api_key_manager: ApiKeyManager,
        rbac_manager: RBACManager,
    ) -> Self {
        Self {
            jwt_manager: Arc::new(jwt_manager),
            api_key_manager: Arc::new(api_key_manager),
            rbac_manager: Arc::new(rbac_manager),
            #[cfg(feature = "db")]
            user_repository: None,
            #[cfg(feature = "db")]
            api_key_repository: None,
            #[cfg(feature = "db")]
            refresh_token_repository: None,
            // PRD-25 Phase 1 — 30 d default; app.rs overrides from SecurityConfig.
            refresh_ttl_secs: 2_592_000,
        }
    }

    /// Set the database client for user, API key, and refresh-token management
    #[cfg(feature = "db")]
    pub fn with_database(mut self, client: DatabaseClient) -> Self {
        self.user_repository = Some(Arc::new(UserRepository::new(client.clone())));
        self.refresh_token_repository =
            Some(Arc::new(RefreshTokenRepository::new(client.clone())));
        self.api_key_repository = Some(Arc::new(ApiKeyRepository::new(client)));
        self
    }

    /// PRD-25 Phase 1 — override the refresh-token TTL (seconds). Additive builder; chains after
    /// `with_database` in `app.rs`. Not `#[cfg(feature = "db")]` so a db-less build still compiles.
    pub fn with_refresh_ttl(mut self, secs: u64) -> Self {
        self.refresh_ttl_secs = secs;
        self
    }

    /// Register a new user
    #[cfg(feature = "db")]
    pub async fn register_user(
        &self,
        credentials: UserCredentials,
    ) -> AuthResult<(String, String)> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Invalid)?;

        // Check if user already exists. Return a distinguishable, message-tagged error
        // (not the opaque `AuthError::Invalid`, whose Display is the legacy "Invalid API
        // key") so the handler's duplicate-detection arms map these to a clean 409 —
        // `handlers.rs::register_user` matches `Internal("...already exists")` →
        // `email_taken` and `Internal("...already taken")` → `username_taken`.
        if let Ok(Some(_)) = user_repo.get_by_email(&credentials.email).await {
            return Err(AuthError::Internal("Email already exists".to_string()));
        }

        if let Ok(Some(_)) = user_repo.get_by_username(&credentials.username).await {
            return Err(AuthError::Internal("Username already taken".to_string()));
        }

        // Hash password
        let password_hash = self.hash_password(&credentials.password);

        // Create user
        let user = User {
            id: None,
            email: credentials.email.clone(),
            username: credentials.username.clone(),
            password_hash,
            tenant_id: credentials.tenant_id,
            roles: vec!["user".to_string()],
            default_project_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            active: true,
            full_name: None,
            display_name: None,
            company: None,
            timezone: None,
            onboarding_completed: false,
        };

        let user_id = user_repo.create(user).await.map_err(|e| {
            error!("Failed to create user: {}", e);
            AuthError::Invalid
        })?;

        info!("User registered: {}", user_id);

        // [PRD-21 Wave A / DR-011 C3] The pre-Wave-2 [OD-011] auto-provisioned, org-less default
        // project (`ProjectRepository::create_default()`) is REMOVED per the ratified
        // "empty-tenant" model: a newly registered user starts with no project and no org.
        // Onboarding (DR-010 / PRD-21 Wave B, not yet built) is the only path that creates a
        // project going forward, and it always does so under an org (`create_named`,
        // `org_id` required). Until Wave B ships, a fresh user has no routable project — see
        // `middleware.rs::load_project_settings`'s doc comment for how that degrades (falls
        // back to `RoutingConfig` server defaults, never fails the request).

        // Generate JWT token
        let token = self
            .jwt_manager
            .create_token(&std::collections::HashMap::from([
                ("sub".to_string(), serde_json::Value::String(user_id.clone())),
                ("email".to_string(), serde_json::Value::String(credentials.email)),
            ]))
            .map_err(|e| {
                error!("Failed to generate token: {}", e);
                AuthError::Invalid
            })?;

        Ok((user_id, token))
    }

    /// Login user with credentials
    #[cfg(feature = "db")]
    pub async fn login_user(&self, credentials: LoginCredentials) -> AuthResult<(User, String)> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Invalid)?;

        // Get user by email
        let user = user_repo
            .get_by_email(&credentials.email)
            .await
            .map_err(|e| {
                error!("Failed to get user: {}", e);
                AuthError::Invalid
            })?
            // Anti-enumeration: a nonexistent email must be byte-for-byte
            // indistinguishable at the API from a wrong password — both map to
            // 401 "Invalid credentials" via the InvalidCredentials arm below.
            // Do not leak "no such user" through a different status/message.
            .ok_or_else(|| AuthError::InvalidCredentials("Invalid email or password".to_string()))?;

        // Check if user is active
        if !user.active {
            warn!("Inactive user attempted login: {}", credentials.email);
            // Tagged message coupled to handlers.rs::login_user's
            // `Internal(msg) if msg.contains("inactive")` arm -> 403 "Account is disabled".
            // Keep the substring "inactive" if this message is ever edited.
            return Err(AuthError::Internal("Account is inactive".to_string()));
        }

        // Verify password using Argon2id (supports legacy bcrypt/SHA-256 hashes for migration)
        if !self.verify_password(&credentials.password, &user.password_hash) {
            warn!("Invalid password for user: {}", credentials.email);
            return Err(AuthError::InvalidCredentials("Invalid password".to_string()));
        }

        info!("User logged in: {}", user.email);

        // DR-010 rehash-on-login: a successful verify against a non-Argon2id hash (legacy
        // bcrypt or SHA-256) transparently migrates the stored hash to Argon2id. Best-effort —
        // a failed update must never fail the login, only be logged.
        if Self::hash_needs_argon2_upgrade(&user.password_hash) {
            if let Some(ref id) = user.id {
                let upgraded_hash = self.hash_password(&credentials.password);
                match user_repo.update_password(id, &upgraded_hash).await {
                    Ok(true) => info!("Rehashed password to argon2id for user: {}", user.email),
                    Ok(false) => warn!(
                        "Rehash-on-login: update_password affected no row for user: {}",
                        user.email
                    ),
                    Err(e) => warn!(
                        "Rehash-on-login: failed to persist argon2id hash for user {}: {}",
                        user.email, e
                    ),
                }
            }
        }

        // Generate JWT token
        let mut claims = std::collections::HashMap::new();
        claims.insert("sub".to_string(), serde_json::Value::String(user.id.clone().unwrap_or_default()));
        claims.insert("email".to_string(), serde_json::Value::String(user.email.clone()));
        if let Some(ref tenant_id) = user.tenant_id {
            claims.insert("tenant_id".to_string(), serde_json::Value::String(tenant_id.clone()));
        }

        let token = self
            .jwt_manager
            .create_token(&claims)
            .map_err(|e| {
                error!("Failed to generate token: {}", e);
                AuthError::Invalid
            })?;

        Ok((user, token))
    }

    /// Validate JWT token and get user
    pub async fn validate_jwt_token(&self, token: &str) -> AuthResult<AuthContext> {
        // Validate token. PRD-25 Phase 1 (BUG-05): propagate the specific error — in particular
        // `AuthError::Expired` must survive so the middleware can flag a refreshable-401 with the
        // `x-gr-token-expired` marker. `validate_token` already returns `Expired` vs `Invalid`;
        // do NOT flatten both to `Invalid` here (the pre-fix behavior that hid expiry).
        let claims = self.jwt_manager.validate_token(token)?;

        // Get user from database if available
        #[cfg(feature = "db")]
        if let Some(user_repo) = &self.user_repository {
            if let Some(sub) = claims.get("sub").and_then(|v| v.as_str()) {
                if let Ok(Some(user)) = user_repo.get_by_id(sub).await {
                    if !user.active {
                        return Err(AuthError::Invalid);
                    }

                    return Ok(AuthContext {
                        api_key: format!("jwt:{}", sub),
                        credential_kind: crate::CredentialKind::Jwt,
                        user_id: user.id,
                        tenant_id: user.tenant_id,
                        permissions: user.roles.clone(),
                        metadata: {
                            let mut map = std::collections::HashMap::new();
                            map.insert(
                                "roles".to_string(),
                                serde_json::json!(user.roles),
                            );
                            map.insert(
                                "email".to_string(),
                                serde_json::Value::String(user.email),
                            );
                            map
                        },
                    });
                }
            }
        }

        // Fallback to token claims if no database
        let sub = claims.get("sub").and_then(|v| v.as_str()).map(String::from);
        let tenant_id = claims.get("tenant_id").and_then(|v| v.as_str()).map(String::from);

        Ok(AuthContext {
            api_key: format!("jwt:{}", sub.as_ref().unwrap_or(&"unknown".to_string())),
            credential_kind: crate::CredentialKind::Jwt,
            user_id: sub,
            tenant_id,
            permissions: vec![],
            metadata: claims,
        })
    }

    /// Validate API key with database lookup
    pub async fn validate_api_key(&self, api_key: &str) -> AuthResult<AuthContext> {
        #[cfg(feature = "db")]
        if let Some(api_key_repo) = &self.api_key_repository {
            // Hash the API key for lookup
            let key_hash = self.hash_api_key(api_key);
            
            // Look up API key in database
            match api_key_repo.get_by_key_hash(&key_hash).await {
                Ok(Some(key_record)) => {
                    // Check if key is active
                    if !key_record.active {
                        warn!("Inactive API key attempted authentication");
                        return Err(AuthError::Invalid);
                    }
                    
                    // Check if key has expired
                    if let Some(expires_at) = key_record.expires_at {
                        if expires_at < chrono::Utc::now() {
                            warn!("Expired API key attempted authentication");
                            return Err(AuthError::Expired);
                        }
                    }
                    
                    // Update last_used timestamp (best-effort, don't fail if it errors)
                    if let Some(ref id) = key_record.id {
                        if let Err(e) = api_key_repo.update_last_used(id).await {
                            warn!("Failed to update API key last_used timestamp: {}", e);
                        }
                    }
                    
                    info!("API key authenticated: {}", key_record.key_prefix);
                    
                    // Build rich auth context
                    let mut metadata = std::collections::HashMap::new();
                    if let Some(ref id) = key_record.id {
                        metadata.insert("api_key_id".to_string(), serde_json::Value::String(id.clone()));
                    }
                    metadata.insert("api_key_prefix".to_string(), serde_json::Value::String(key_record.key_prefix.clone()));
                    
                    // Get rate limit info
                    if let Some(rpm) = key_record.rate_limit_per_minute {
                        metadata.insert("rate_limit_per_minute".to_string(), serde_json::Value::Number(rpm.into()));
                    }
                    if let Some(rpd) = key_record.rate_limit_per_day {
                        metadata.insert("rate_limit_per_day".to_string(), serde_json::Value::Number(rpd.into()));
                    }
                    
                    return Ok(AuthContext {
                        api_key: api_key.to_string(),
                        credential_kind: crate::CredentialKind::ApiKey,
                        user_id: Some(key_record.user_id),
                        tenant_id: key_record.tenant_id,
                        permissions: vec![], // Can be extended with key-specific permissions
                        metadata,
                    });
                }
                Ok(None) => {
                    warn!("API key not found in database");
                    return Err(AuthError::Invalid);
                }
                Err(e) => {
                    error!("Failed to lookup API key in database: {}", e);
                    return Err(AuthError::Invalid);
                }
            }
        }
        
        // Fallback for non-database mode (basic validation only)
        Ok(AuthContext {
            api_key: api_key.to_string(),
            credential_kind: crate::CredentialKind::ApiKey,
            user_id: None,
            tenant_id: None,
            permissions: vec![],
            metadata: std::collections::HashMap::new(),
        })
    }
    
    /// Create a new API key for a user. `project_id` (DR-012 — API-key project scoping) is the
    /// owning project the key resolves to at request time; `None` for an unscoped key (legacy
    /// shape — resolution then falls through to the deterministic org-membership safety net).
    #[cfg(feature = "db")]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_api_key(
        &self,
        user_id: &str,
        name: Option<String>,
        tenant_id: Option<String>,
        project_id: Option<String>,
        rate_limit_per_minute: Option<u32>,
        rate_limit_per_day: Option<u32>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AuthResult<(String, String)> {
        let api_key_repo = self
            .api_key_repository
            .as_ref()
            .ok_or_else(|| AuthError::Invalid)?;
        
        // Generate a new API key (32 random bytes = 64 hex chars)
        let raw_key = self.generate_api_key();
        let key_hash = self.hash_api_key(&raw_key);
        let key_prefix = raw_key.chars().take(8).collect::<String>();
        
        // Create API key record
        let api_key = ApiKey {
            id: None,
            key_hash,
            key_prefix,
            user_id: user_id.to_string(),
            tenant_id,
            project_id,
            name,
            rate_limit_per_minute,
            rate_limit_per_day,
            created_at: chrono::Utc::now(),
            expires_at,
            last_used_at: None,
            active: true,
        };
        
        let key_id = api_key_repo.create(api_key).await.map_err(|e| {
            error!("Failed to create API key: {}", e);
            AuthError::Invalid
        })?;
        
        info!("API key created for user {}: {}", user_id, key_id);
        
        // Return the key ID and the raw key (this is the only time the raw key is available)
        Ok((key_id, raw_key))
    }
    
    // ─── PRD-25 Phase 1 — refresh tokens (BUG-05: closes the "zombie session") ──────────
    //
    // Opaque, rotating refresh tokens. Reuses the existing API-key primitives verbatim — the
    // 32-byte→64-hex RNG (`generate_api_key`) mints both the raw token and its `family_id`, and
    // the SHA-256 hasher (`hash_api_key`) hashes at rest. No new crypto/RNG is introduced.

    /// Issue a fresh refresh token for `user_id`, starting a new family. Returns the RAW token
    /// (the only time it is available; stored hashed). Requires the DB — errors if no repo.
    #[cfg(feature = "db")]
    pub async fn issue_refresh_token(&self, user_id: &str) -> AuthResult<String> {
        let repo = self
            .refresh_token_repository
            .as_ref()
            .ok_or_else(|| AuthError::Invalid)?;

        let raw = self.generate_api_key(); // 32 random bytes → 64 hex (reused primitive)
        let token_hash = self.hash_api_key(&raw); // SHA-256 at rest (reused primitive)
        let family_id = self.generate_api_key(); // opaque family grouping id (reused RNG)
        let expires_at =
            chrono::Utc::now() + chrono::Duration::seconds(self.refresh_ttl_secs as i64);

        repo.create(&token_hash, user_id, &family_id, expires_at)
            .await
            .map_err(|e| {
                error!("Failed to persist refresh token: {}", e);
                AuthError::Invalid
            })?;

        Ok(raw)
    }

    /// Rotate a presented raw refresh token, per the PRD-25 §refresh-contract algorithm:
    /// exists → not-expired → not-revoked (else reuse → revoke family) → active user →
    /// atomic single-use rotate → mint a fresh access JWT + successor refresh row. Returns the
    /// new `{ user, access_token, refresh_token }` on success, or a typed `RefreshError` the
    /// handler maps to a 401 `code`.
    #[cfg(feature = "db")]
    pub async fn rotate_refresh_token(&self, raw: &str) -> Result<RefreshRotation, RefreshError> {
        let repo = self
            .refresh_token_repository
            .as_ref()
            .ok_or_else(|| RefreshError::Internal("refresh repository not configured".to_string()))?;
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| RefreshError::Internal("user repository not configured".to_string()))?;

        let old_hash = self.hash_api_key(raw);

        // 1. exists?
        let record = repo
            .get_by_token_hash(&old_hash)
            .await
            .map_err(|e| RefreshError::Internal(e.to_string()))?
            .ok_or(RefreshError::InvalidGrant)?;

        // 2-3. Classify the record's security state (expiry + revocation/reuse) via the pure,
        // unit-tested `classify_refresh_record`. Behavior is identical to the previous inline
        // logic — expiry first, then grace-vs-reuse on a revoked row.
        match classify_refresh_record(
            record.revoked_at,
            record.replaced_by.clone(),
            Some(record.expires_at),
            chrono::Utc::now(),
            REFRESH_GRACE_SECS,
        ) {
            RefreshRecordDecision::Expired => return Err(RefreshError::Expired),
            // A benign concurrent-rotation race — reject without nuking the family.
            RefreshRecordDecision::BenignGraceRetry => return Err(RefreshError::InvalidGrant),
            // Genuine reuse — revoke the whole family (best-effort) and report theft.
            RefreshRecordDecision::ReuseDetected => {
                if let Err(e) = repo.revoke_family(&record.family_id).await {
                    warn!("refresh reuse: family revoke failed (non-fatal): {}", e);
                }
                return Err(RefreshError::ReuseDetected);
            }
            // Live token — fall through to the active-user check + atomic rotate.
            RefreshRecordDecision::Active => {}
        }

        // 4. active user?
        let user = user_repo
            .get_by_id(&record.user_id)
            .await
            .map_err(|e| RefreshError::Internal(e.to_string()))?
            .filter(|u| u.active)
            .ok_or(RefreshError::InvalidGrant)?;

        // 5. atomic single-use rotate. 0 rows = another rotation won the race (lost race =
        // benign, no family nuke).
        let new_raw = self.generate_api_key();
        let new_hash = self.hash_api_key(&new_raw);
        let rotated = repo
            .rotate_if_active(&old_hash, &new_hash)
            .await
            .map_err(|e| RefreshError::Internal(e.to_string()))?;
        if !rotated {
            return Err(RefreshError::InvalidGrant);
        }

        // 6. create the successor row, carrying the family forward with a fresh expiry.
        let expires_at =
            chrono::Utc::now() + chrono::Duration::seconds(self.refresh_ttl_secs as i64);
        if let Err(e) = repo
            .create(&new_hash, &record.user_id, &record.family_id, expires_at)
            .await
        {
            // The old token is already revoked at this point; failing to persist the successor
            // leaves the family without an active token (the caller must log in again). Report
            // as infrastructure failure rather than a grant/reuse problem.
            return Err(RefreshError::Internal(e.to_string()));
        }

        // 7. mint a fresh access JWT (same claim shape as login_user).
        let mut claims = std::collections::HashMap::new();
        claims.insert(
            "sub".to_string(),
            serde_json::Value::String(user.id.clone().unwrap_or_default()),
        );
        claims.insert(
            "email".to_string(),
            serde_json::Value::String(user.email.clone()),
        );
        if let Some(ref tenant_id) = user.tenant_id {
            claims.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id.clone()),
            );
        }
        let access_token = self
            .jwt_manager
            .create_token(&claims)
            .map_err(|e| RefreshError::Internal(e.to_string()))?;

        Ok(RefreshRotation {
            user,
            access_token,
            refresh_token: new_raw,
        })
    }

    /// Revoke the family that owns the given raw refresh token (logout — DB-backed, so it works
    /// even when Redis is down). A missing/unknown token is a silent no-op.
    #[cfg(feature = "db")]
    pub async fn revoke_refresh_family_by_token(&self, raw: &str) -> AuthResult<()> {
        let repo = self
            .refresh_token_repository
            .as_ref()
            .ok_or_else(|| AuthError::Invalid)?;
        let hash = self.hash_api_key(raw);
        if let Some(record) = repo.get_by_token_hash(&hash).await.map_err(|e| {
            warn!("logout: refresh lookup failed: {}", e);
            AuthError::Invalid
        })? {
            repo.revoke_family(&record.family_id).await.map_err(|e| {
                warn!("logout: refresh family revoke failed: {}", e);
                AuthError::Invalid
            })?;
        }
        Ok(())
    }

    /// Delete every refresh token for a user (account deactivation / deletion teardown).
    #[cfg(feature = "db")]
    pub async fn revoke_user_refresh_tokens(&self, user_id: &str) -> AuthResult<()> {
        let repo = self
            .refresh_token_repository
            .as_ref()
            .ok_or_else(|| AuthError::Invalid)?;
        repo.delete_for_user(user_id).await.map_err(|e| {
            warn!("failed to delete refresh tokens for user {}: {}", user_id, e);
            AuthError::Invalid
        })
    }

    /// Revoke (deactivate) an API key. Only the key's owner may revoke it (OD-009).
    #[cfg(feature = "db")]
    pub async fn revoke_api_key(&self, key_id: &str, requesting_user_id: &str) -> AuthResult<()> {
        let api_key_repo = self
            .api_key_repository
            .as_ref()
            .ok_or_else(|| AuthError::Invalid)?;

        // Get the key
        let mut key = api_key_repo
            .get_by_id(key_id)
            .await
            .map_err(|e| {
                error!("Failed to get API key: {}", e);
                AuthError::Invalid
            })?
            .ok_or_else(|| AuthError::Invalid)?;

        check_key_ownership(&key, requesting_user_id)?;

        // Deactivate it
        key.active = false;

        api_key_repo
            .update(key_id, key)
            .await
            .map_err(|e| {
                error!("Failed to revoke API key: {}", e);
                AuthError::Invalid
            })?;

        info!("API key revoked: {} by user {}", key_id, requesting_user_id);
        Ok(())
    }
    
    /// List API keys for a user
    #[cfg(feature = "db")]
    pub async fn list_user_api_keys(&self, user_id: &str) -> AuthResult<Vec<ApiKey>> {
        let api_key_repo = self
            .api_key_repository
            .as_ref()
            .ok_or_else(|| AuthError::Invalid)?;
        
        api_key_repo
            .get_by_user_id(user_id)
            .await
            .map_err(|e| {
                error!("Failed to list API keys: {}", e);
                AuthError::Invalid
            })
    }

    /// Check if user has permission
    pub fn check_permission(
        &self,
        auth_context: &AuthContext,
        resource: &Resource,
        permission: &Permission,
    ) -> AuthResult<bool> {
        let access_context = AccessContext {
            user_id: auth_context
                .user_id
                .clone()
                .unwrap_or_else(|| "anonymous".to_string()),
            tenant_id: auth_context.tenant_id.clone(),
            roles: auth_context.permissions.clone(),
            ip_address: None,
            metadata: std::collections::HashMap::new(),
        };

        self.rbac_manager
            .check_permission(&access_context, resource, permission)
            .map_err(|_| AuthError::Invalid)
    }

    /// Get user by ID
    #[cfg(feature = "db")]
    pub async fn get_user(&self, user_id: &str) -> AuthResult<Option<User>> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Invalid)?;

        user_repo
            .get_by_id(user_id)
            .await
            .map_err(|_| AuthError::Invalid)
    }

    /// Hash password using Argon2id (DR-010, ratified 2026-07-16 — "same bar as
    /// `project.access_secret`"; mirrors `gaussmeridian_utils::EncryptionUtils::hash_secret_argon2id`
    /// and `gaussmoa::security::KeyManager::hash_password`). Uses the `argon2` crate's
    /// recommended defaults (Argon2id, version 0x13), PHC string output (`$argon2id$...`).
    fn hash_password(&self, password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .unwrap_or_else(|e| {
                error!("Failed to hash password with argon2id: {}", e);
                // Fallback to bcrypt if argon2id itself fails (shouldn't happen)
                bcrypt::hash(password, BCRYPT_COST).unwrap_or_else(|e2| {
                    error!("Failed to hash password with bcrypt fallback: {}", e2);
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(password.as_bytes());
                    format!("{:x}", hasher.finalize())
                })
            })
    }

    /// Verify password against a stored hash, detecting the hash format so rows written
    /// before the DR-010 Argon2id migration keep verifying:
    /// - `$argon2...` → Argon2id (current format, via `hash_password`)
    /// - `$2a$` / `$2b$` / `$2y$` → legacy bcrypt
    /// - anything else → legacy SHA-256 (pre-bcrypt rows)
    ///
    /// Callers that want to migrate a successfully-verified legacy hash to Argon2id should
    /// check `hash_needs_argon2_upgrade` afterward (see `login_user`'s rehash-on-login step).
    fn verify_password(&self, password: &str, hash: &str) -> bool {
        if hash.starts_with("$argon2") {
            return match PasswordHash::new(hash) {
                Ok(parsed) => Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok(),
                Err(e) => {
                    warn!("Malformed argon2id hash encountered during verify: {}", e);
                    false
                }
            };
        }

        if hash.starts_with("$2a$") || hash.starts_with("$2b$") || hash.starts_with("$2y$") {
            return bcrypt::verify(password, hash).unwrap_or(false);
        }

        // Legacy SHA-256 hash (pre-bcrypt rows)
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let sha_hash = format!("{:x}", hasher.finalize());
        sha_hash == hash
    }

    /// DR-010 rehash-on-login seam: true when `hash` is not already an Argon2id PHC string,
    /// i.e. a legacy bcrypt or SHA-256 hash that should be upgraded after the next successful
    /// verify. Pure and DB-independent so it's unit-testable without a repository.
    fn hash_needs_argon2_upgrade(hash: &str) -> bool {
        !hash.starts_with("$argon2")
    }
    
    /// Hash API key using SHA-256 (API keys don't need bcrypt as they're random).
    /// `pub`: also used by `update_project_settings` (OD-008) to resolve a caller's
    /// project from the same raw credential `extract_auth_context` already validated,
    /// rather than re-deriving the hash a third time (middleware.rs's `load_project_settings`
    /// is the second, pre-existing copy of this exact logic).
    pub fn hash_api_key(&self, api_key: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Generate a new API key (32 random bytes = 64 hex chars)
    fn generate_api_key(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let key_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        key_bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
    
    /// Generate a password reset token
    pub fn generate_reset_token(&self, user_id: &str) -> PasswordResetToken {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let token_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        let token: String = token_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        
        PasswordResetToken {
            token,
            user_id: user_id.to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1), // 1 hour expiry
        }
    }
    
    /// Change user password
    #[cfg(feature = "db")]
    pub async fn change_password(
        &self,
        user_id: &str,
        request: PasswordChangeRequest,
    ) -> AuthResult<()> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Internal("Database not configured".to_string()))?;
        
        // Get user
        let user = user_repo
            .get_by_id(user_id)
            .await
            .map_err(|e| AuthError::Internal(format!("Database error: {}", e)))?
            .ok_or_else(|| AuthError::InvalidCredentials("User not found".to_string()))?;
        
        // Verify current password
        if !self.verify_password(&request.current_password, &user.password_hash) {
            return Err(AuthError::InvalidCredentials("Current password is incorrect".to_string()));
        }
        
        // Hash new password
        let new_hash = self.hash_password(&request.new_password);
        
        // Update password
        user_repo
            .update_password(user_id, &new_hash)
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to update password: {}", e)))?;
        
        info!("Password changed for user: {}", user_id);
        Ok(())
    }
    
    /// Reset password with token (for forgot password flow)
    #[cfg(feature = "db")]
    pub async fn reset_password_with_token(
        &self,
        user_id: &str,
        new_password: &str,
    ) -> AuthResult<()> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Internal("Database not configured".to_string()))?;
        
        // Hash new password
        let new_hash = self.hash_password(new_password);
        
        // Update password
        let updated = user_repo
            .update_password(user_id, &new_hash)
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to reset password: {}", e)))?;
        if !updated {
            return Err(AuthError::Internal(
                "password reset affected no user record".to_string(),
            ));
        }

        info!("Password reset for user: {}", user_id);
        Ok(())
    }
    
    /// Update user profile
    #[cfg(feature = "db")]
    pub async fn update_profile(
        &self,
        user_id: &str,
        update: UserProfileUpdate,
    ) -> AuthResult<User> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Internal("Database not configured".to_string()))?;
        
        // Get current user
        let mut user = user_repo
            .get_by_id(user_id)
            .await
            .map_err(|e| AuthError::Internal(format!("Database error: {}", e)))?
            .ok_or_else(|| AuthError::InvalidCredentials("User not found".to_string()))?;
        
        // Check for email uniqueness if changing email
        if let Some(ref new_email) = update.email {
            if new_email != &user.email {
                if let Ok(Some(_)) = user_repo.get_by_email(new_email).await {
                    return Err(AuthError::InvalidCredentials("Email already in use".to_string()));
                }
                user.email = new_email.clone();
            }
        }
        
        // Check for username uniqueness if changing username
        if let Some(ref new_username) = update.username {
            if new_username != &user.username {
                if let Ok(Some(_)) = user_repo.get_by_username(new_username).await {
                    return Err(AuthError::InvalidCredentials("Username already in use".to_string()));
                }
                user.username = new_username.clone();
            }
        }
        
        // Update user
        let updated_user = user_repo
            .update(user_id, user.clone())
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to update user: {}", e)))?
            .ok_or_else(|| AuthError::Internal("Failed to get updated user".to_string()))?;
        
        info!("Profile updated for user: {}", user_id);
        Ok(updated_user)
    }
    
    /// Deactivate a user account
    #[cfg(feature = "db")]
    pub async fn deactivate_user(&self, user_id: &str) -> AuthResult<()> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Internal("Database not configured".to_string()))?;
        
        user_repo
            .set_active(user_id, false)
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to deactivate user: {}", e)))?;
        
        info!("User deactivated: {}", user_id);
        Ok(())
    }
    
    /// Reactivate a user account
    #[cfg(feature = "db")]
    pub async fn reactivate_user(&self, user_id: &str) -> AuthResult<()> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Internal("Database not configured".to_string()))?;
        
        user_repo
            .set_active(user_id, true)
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to reactivate user: {}", e)))?;
        
        info!("User reactivated: {}", user_id);
        Ok(())
    }
    
    /// Update user roles
    #[cfg(feature = "db")]
    pub async fn update_user_roles(&self, user_id: &str, roles: Vec<String>) -> AuthResult<()> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Internal("Database not configured".to_string()))?;
        
        user_repo
            .update_roles(user_id, roles.clone())
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to update roles: {}", e)))?;
        
        info!("Roles updated for user {}: {:?}", user_id, roles);
        Ok(())
    }
    
    /// List all users (admin function)
    #[cfg(feature = "db")]
    pub async fn list_users(&self, limit: Option<u32>) -> AuthResult<Vec<User>> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Internal("Database not configured".to_string()))?;
        
        user_repo
            .list_all(limit)
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to list users: {}", e)))
    }
    
    /// Get user by email (for password reset flow)
    #[cfg(feature = "db")]
    pub async fn get_user_by_email(&self, email: &str) -> AuthResult<Option<User>> {
        let user_repo = self
            .user_repository
            .as_ref()
            .ok_or_else(|| AuthError::Internal("Database not configured".to_string()))?;
        
        user_repo
            .get_by_email(email)
            .await
            .map_err(|e| AuthError::Internal(format!("Database error: {}", e)))
    }
}

/// Guard for OD-009: only the key's owner may act on it. Pure and DB-independent
/// so it's testable without a repository or connection.
#[cfg(feature = "db")]
fn check_key_ownership(key: &ApiKey, requesting_user_id: &str) -> AuthResult<()> {
    if key.user_id != requesting_user_id {
        return Err(AuthError::InsufficientPermissions);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let manager = AuthManager::new(
            JwtManager::new("test_secret"),
            ApiKeyManager::new(),
            RBACManager::new(),
        );

        let password = "password123";
        let wrong_password = "wrong_password";

        // Hash the password
        let hash = manager.hash_password(password);

        // argon2id produces different hashes for the same input (due to random salt)
        // So we verify by checking that the password verifies against the hash
        assert!(manager.verify_password(password, &hash));
        assert!(!manager.verify_password(wrong_password, &hash));
    }

    // ─── DR-010 — Argon2id migration ────────────────────────────────────────────────

    #[test]
    fn hash_password_produces_an_argon2id_phc_string() {
        let manager = AuthManager::new(
            JwtManager::new("test_secret"),
            ApiKeyManager::new(),
            RBACManager::new(),
        );

        let hash = manager.hash_password("password123");
        assert!(
            hash.starts_with("$argon2id$"),
            "new password hashes must use the Argon2id variant, got: {}",
            hash
        );
    }

    #[test]
    fn verify_password_accepts_a_legacy_bcrypt_hash() {
        let manager = AuthManager::new(
            JwtManager::new("test_secret"),
            ApiKeyManager::new(),
            RBACManager::new(),
        );

        let password = "password123";
        // A hash produced the old way (bcrypt), as an existing dev/prod row would hold.
        let legacy_hash = bcrypt::hash(password, BCRYPT_COST).expect("bcrypt hash succeeds");

        assert!(manager.verify_password(password, &legacy_hash));
        assert!(!manager.verify_password("wrong_password", &legacy_hash));
    }

    #[test]
    fn verify_password_accepts_a_legacy_sha256_hash() {
        let manager = AuthManager::new(
            JwtManager::new("test_secret"),
            ApiKeyManager::new(),
            RBACManager::new(),
        );

        let password = "password123";
        // A hash produced by the pre-bcrypt legacy path (plain SHA-256 hex digest).
        let legacy_hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(password.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        assert!(manager.verify_password(password, &legacy_hash));
        assert!(!manager.verify_password("wrong_password", &legacy_hash));
    }

    #[test]
    fn hash_needs_argon2_upgrade_flags_only_non_argon2_hashes() {
        let argon2_hash = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$aGFzaA";
        let bcrypt_hash = "$2b$12$abcdefghijklmnopqrstuv";
        let sha256_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d";

        assert!(!AuthManager::hash_needs_argon2_upgrade(argon2_hash));
        assert!(AuthManager::hash_needs_argon2_upgrade(bcrypt_hash));
        assert!(AuthManager::hash_needs_argon2_upgrade(sha256_hash));
    }

    #[test]
    fn test_password_hashing_produces_different_hashes() {
        let manager = AuthManager::new(
            JwtManager::new("test_secret"),
            ApiKeyManager::new(),
            RBACManager::new(),
        );

        let password = "password123";

        // argon2id produces different hashes for the same input due to random salting
        let hash1 = manager.hash_password(password);
        let hash2 = manager.hash_password(password);

        // Hashes should be different (different salts)
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(manager.verify_password(password, &hash1));
        assert!(manager.verify_password(password, &hash2));
    }

    #[test]
    fn test_api_key_generation() {
        let manager = AuthManager::new(
            JwtManager::new("test_secret"),
            ApiKeyManager::new(),
            RBACManager::new(),
        );

        let key1 = manager.generate_api_key();
        let key2 = manager.generate_api_key();
        
        // Keys should be different
        assert_ne!(key1, key2);
        
        // Keys should be 64 characters (32 bytes hex encoded)
        assert_eq!(key1.len(), 64);
        assert_eq!(key2.len(), 64);
    }

    #[test]
    fn test_api_key_hashing() {
        let manager = AuthManager::new(
            JwtManager::new("test_secret"),
            ApiKeyManager::new(),
            RBACManager::new(),
        );

        let api_key = "test_api_key_12345";
        
        // API key hashing uses SHA-256, which is deterministic
        let hash1 = manager.hash_api_key(api_key);
        let hash2 = manager.hash_api_key(api_key);
        
        assert_eq!(hash1, hash2);
        
        // Different key should produce different hash
        let hash3 = manager.hash_api_key("different_key");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_reset_token_generation() {
        let manager = AuthManager::new(
            JwtManager::new("test_secret"),
            ApiKeyManager::new(),
            RBACManager::new(),
        );

        let token1 = manager.generate_reset_token("user123");
        let token2 = manager.generate_reset_token("user123");
        
        // Tokens should be different even for same user
        assert_ne!(token1.token, token2.token);
        
        // User ID should be set correctly
        assert_eq!(token1.user_id, "user123");
        
        // Token should expire in the future
        assert!(token1.expires_at > chrono::Utc::now());
    }

    #[cfg(feature = "db")]
    fn sample_key(user_id: &str) -> ApiKey {
        ApiKey {
            id: None,
            key_hash: "hash".to_string(),
            key_prefix: "grk_test".to_string(),
            user_id: user_id.to_string(),
            tenant_id: None,
            project_id: None,
            name: None,
            rate_limit_per_minute: None,
            rate_limit_per_day: None,
            created_at: chrono::Utc::now(),
            expires_at: None,
            last_used_at: None,
            active: true,
        }
    }

    // OD-009: revoke_api_key must reject a caller who doesn't own the key.
    #[cfg(feature = "db")]
    #[test]
    fn check_key_ownership_allows_the_owner() {
        let key = sample_key("user_a");
        assert!(check_key_ownership(&key, "user_a").is_ok());
    }

    #[cfg(feature = "db")]
    #[test]
    fn check_key_ownership_rejects_a_non_owner() {
        let key = sample_key("user_a");
        let result = check_key_ownership(&key, "user_b");
        assert!(matches!(result, Err(AuthError::InsufficientPermissions)));
    }

    // ─── PRD-25 Phase 1 — refresh-record classification (crown-jewel decision) ──────────
    //
    // Branch-covers `classify_refresh_record`, the pure security decision extracted from
    // `rotate_refresh_token`. The atomic SQL (rotate → 0 rows, revoke_family) stays covered by
    // the mem-engine tests in `gaussmeridian-db::refresh_token_repository`.

    #[cfg(feature = "db")]
    const GRACE: i64 = REFRESH_GRACE_SECS;

    #[cfg(feature = "db")]
    #[test]
    fn classify_expired_when_expires_at_in_the_past() {
        let now = chrono::Utc::now();
        let decision = classify_refresh_record(
            None,                                     // not revoked
            None,                                     // no successor
            Some(now - chrono::Duration::seconds(1)), // expired 1s ago
            now,
            GRACE,
        );
        assert_eq!(decision, RefreshRecordDecision::Expired);
    }

    #[cfg(feature = "db")]
    #[test]
    fn classify_expired_when_expires_at_is_none() {
        // A record with no lifetime recorded is treated as expired (mirrors the SCHEMAFULL
        // `option<datetime>` field being NONE — an unusable token, never Active).
        let now = chrono::Utc::now();
        let decision = classify_refresh_record(None, None, None, now, GRACE);
        assert_eq!(decision, RefreshRecordDecision::Expired);
    }

    #[cfg(feature = "db")]
    #[test]
    fn classify_active_when_not_revoked_and_not_expired() {
        let now = chrono::Utc::now();
        let decision = classify_refresh_record(
            None,                                      // not revoked
            None,
            Some(now + chrono::Duration::days(30)),    // valid
            now,
            GRACE,
        );
        assert_eq!(decision, RefreshRecordDecision::Active);
    }

    #[cfg(feature = "db")]
    #[test]
    fn classify_reuse_when_revoked_older_than_grace() {
        let now = chrono::Utc::now();
        let decision = classify_refresh_record(
            Some(now - chrono::Duration::seconds(GRACE + 5)), // revoked well outside grace
            Some("successor_hash".to_string()),               // successor present, but stale
            Some(now + chrono::Duration::days(30)),           // not expired
            now,
            GRACE,
        );
        assert_eq!(decision, RefreshRecordDecision::ReuseDetected);
    }

    #[cfg(feature = "db")]
    #[test]
    fn classify_benign_grace_when_revoked_within_grace_and_replaced_by_some() {
        let now = chrono::Utc::now();
        let decision = classify_refresh_record(
            Some(now - chrono::Duration::seconds(2)), // revoked 2s ago → within grace
            Some("successor_hash".to_string()),       // successor present
            Some(now + chrono::Duration::days(30)),   // not expired
            now,
            GRACE,
        );
        assert_eq!(decision, RefreshRecordDecision::BenignGraceRetry);
    }

    #[cfg(feature = "db")]
    #[test]
    fn classify_reuse_when_revoked_within_grace_but_replaced_by_none() {
        // Revoked recently but with NO successor recorded → not a benign rotation race; theft.
        let now = chrono::Utc::now();
        let decision = classify_refresh_record(
            Some(now - chrono::Duration::seconds(2)), // within grace
            None,                                     // but no successor
            Some(now + chrono::Duration::days(30)),   // not expired
            now,
            GRACE,
        );
        assert_eq!(decision, RefreshRecordDecision::ReuseDetected);
    }

    #[cfg(feature = "db")]
    #[test]
    fn classify_expired_takes_precedence_over_revocation() {
        // A revoked-and-expired token is Expired (benign), never ReuseDetected — the ordering
        // guarantee `rotate_refresh_token` relies on.
        let now = chrono::Utc::now();
        let decision = classify_refresh_record(
            Some(now - chrono::Duration::seconds(1)), // revoked recently
            Some("successor_hash".to_string()),
            Some(now - chrono::Duration::seconds(1)), // but also expired
            now,
            GRACE,
        );
        assert_eq!(decision, RefreshRecordDecision::Expired);
    }
}

