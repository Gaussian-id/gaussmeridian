//! OAuth2 authentication provider
//!
//! This module provides OAuth2 authentication support for GaussMeridian,
//! including authorization code flow, token management, and refresh tokens.

use crate::error::AuthError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use url::Url;

/// OAuth2 provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: Option<String>,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub issuer: Option<String>,
}

/// OAuth2 token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub id_token: Option<String>,
}

/// OAuth2 token storage
#[derive(Debug, Clone)]
pub struct OAuth2Token {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub token_type: String,
    pub scopes: Vec<String>,
    pub user_id: Option<String>,
}

impl OAuth2Token {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() >= expires_at
        } else {
            false
        }
    }
}

/// OAuth2 provider implementation
pub struct OAuth2Provider {
    config: OAuth2Config,
    http_client: reqwest::Client,
}

impl OAuth2Provider {
    /// Create a new OAuth2 provider
    pub fn new(config: OAuth2Config) -> Result<Self, AuthError> {
        // Validate URLs
        Url::parse(&config.authorization_endpoint).map_err(|e| {
            AuthError::InvalidConfig(format!("Invalid authorization endpoint: {}", e))
        })?;
        Url::parse(&config.token_endpoint)
            .map_err(|e| AuthError::InvalidConfig(format!("Invalid token endpoint: {}", e)))?;
        if let Some(ref userinfo) = config.userinfo_endpoint {
            Url::parse(userinfo).map_err(|e| {
                AuthError::InvalidConfig(format!("Invalid userinfo endpoint: {}", e))
            })?;
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AuthError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Generate authorization URL
    pub fn authorization_url(&self, state: &str) -> Result<Url, AuthError> {
        let mut url = Url::parse(&self.config.authorization_endpoint).map_err(|e| {
            AuthError::InvalidConfig(format!("Invalid authorization endpoint: {}", e))
        })?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &self.config.scopes.join(" "))
            .append_pair("state", state);

        Ok(url)
    }

    /// Exchange authorization code for tokens
    ///
    /// The `state` parameter is accepted for API completeness and potential
    /// future CSRF validation, but is currently unused.
    pub async fn exchange_code(&self, code: &str, _state: &str) -> Result<OAuth2Token, AuthError> {
        info!("Exchanging authorization code for tokens");

        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", &self.config.redirect_uri);
        params.insert("client_id", &self.config.client_id);
        params.insert("client_secret", &self.config.client_secret);

        let response = self
            .http_client
            .post(&self.config.token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::Network(format!("Token exchange failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AuthError::InvalidCredentials(format!(
                "Token exchange failed with status {}: {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            AuthError::InvalidResponse(format!("Failed to parse token response: {}", e))
        })?;

        let expires_at = token_response
            .expires_in
            .map(|expires_in| Utc::now() + Duration::seconds(expires_in as i64));

        let scopes = token_response
            .scope
            .map(|s| s.split(' ').map(String::from).collect())
            .unwrap_or_else(|| self.config.scopes.clone());

        let token = OAuth2Token {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
            token_type: token_response.token_type,
            scopes,
            user_id: None,
        };

        // If userinfo endpoint is configured, fetch user information
        if let Some(ref _userinfo_endpoint) = self.config.userinfo_endpoint {
            if let Ok(user_info) = self.fetch_user_info(&token.access_token).await {
                // Extract user ID from user info (common fields: sub, id, user_id)
                if let Some(sub) = user_info.get("sub") {
                    if let Some(sub_str) = sub.as_str() {
                        // Store user_id in token for later use
                        debug!("Fetched user info: sub={}", sub_str);
                    }
                }
            }
        }

        Ok(token)
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuth2Token, AuthError> {
        info!("Refreshing access token");

        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);
        params.insert("client_id", &self.config.client_id);
        params.insert("client_secret", &self.config.client_secret);

        let response = self
            .http_client
            .post(&self.config.token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::Network(format!("Token refresh failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AuthError::InvalidCredentials(format!(
                "Token refresh failed with status {}: {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            AuthError::InvalidResponse(format!("Failed to parse token response: {}", e))
        })?;

        let expires_at = token_response
            .expires_in
            .map(|expires_in| Utc::now() + Duration::seconds(expires_in as i64));

        let scopes = token_response
            .scope
            .map(|s| s.split(' ').map(String::from).collect())
            .unwrap_or_else(|| self.config.scopes.clone());

        Ok(OAuth2Token {
            access_token: token_response.access_token,
            refresh_token: token_response
                .refresh_token
                .or(Some(refresh_token.to_string())),
            expires_at,
            token_type: token_response.token_type,
            scopes,
            user_id: None,
        })
    }

    /// Validate access token
    pub async fn validate_token(&self, access_token: &str) -> Result<bool, AuthError> {
        // If userinfo endpoint is configured, use it for validation
        if self.config.userinfo_endpoint.is_some() {
            match self.fetch_user_info(access_token).await {
                Ok(_) => Ok(true),
                Err(e) => {
                    debug!("Token validation failed: {}", e);
                    Ok(false)
                }
            }
        } else {
            // Without userinfo endpoint, we can't validate the token
            // This is a limitation - in production, use introspection endpoint or JWT validation
            warn!("Token validation requested but no userinfo endpoint configured");
            Ok(true) // Assume valid if we can't check
        }
    }

    /// Fetch user information using access token
    async fn fetch_user_info(
        &self,
        access_token: &str,
    ) -> Result<HashMap<String, serde_json::Value>, AuthError> {
        let userinfo_endpoint = self.config.userinfo_endpoint.as_ref().ok_or_else(|| {
            AuthError::InvalidConfig("Userinfo endpoint not configured".to_string())
        })?;

        let response = self
            .http_client
            .get(userinfo_endpoint)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| AuthError::Network(format!("Failed to fetch user info: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AuthError::InvalidCredentials(format!(
                "Userinfo request failed with status {}",
                status
            )));
        }

        let user_info: HashMap<String, serde_json::Value> = response
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(format!("Failed to parse user info: {}", e)))?;

        Ok(user_info)
    }

    /// Get user information from access token
    pub async fn get_user_info(
        &self,
        access_token: &str,
    ) -> Result<HashMap<String, serde_json::Value>, AuthError> {
        self.fetch_user_info(access_token).await
    }
}

/// OAuth2 manager for handling multiple providers
pub struct OAuth2Manager {
    providers: HashMap<String, OAuth2Provider>,
}

impl OAuth2Manager {
    /// Create a new OAuth2 manager
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register an OAuth2 provider
    pub fn register_provider(
        &mut self,
        name: String,
        config: OAuth2Config,
    ) -> Result<(), AuthError> {
        let provider = OAuth2Provider::new(config)?;
        self.providers.insert(name, provider);
        Ok(())
    }

    /// Get a provider by name
    pub fn get_provider(&self, name: &str) -> Option<&OAuth2Provider> {
        self.providers.get(name)
    }

    /// Generate authorization URL for a provider
    pub fn authorization_url(&self, provider_name: &str, state: &str) -> Result<Url, AuthError> {
        let provider = self.providers.get(provider_name).ok_or_else(|| {
            AuthError::InvalidConfig(format!("OAuth2 provider '{}' not found", provider_name))
        })?;
        provider.authorization_url(state)
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        provider_name: &str,
        code: &str,
        state: &str,
    ) -> Result<OAuth2Token, AuthError> {
        let provider = self.providers.get(provider_name).ok_or_else(|| {
            AuthError::InvalidConfig(format!("OAuth2 provider '{}' not found", provider_name))
        })?;
        provider.exchange_code(code, state).await
    }

    /// Refresh token for a provider
    pub async fn refresh_token(
        &self,
        provider_name: &str,
        refresh_token: &str,
    ) -> Result<OAuth2Token, AuthError> {
        let provider = self.providers.get(provider_name).ok_or_else(|| {
            AuthError::InvalidConfig(format!("OAuth2 provider '{}' not found", provider_name))
        })?;
        provider.refresh_token(refresh_token).await
    }

    /// Validate token for a provider
    pub async fn validate_token(
        &self,
        provider_name: &str,
        access_token: &str,
    ) -> Result<bool, AuthError> {
        let provider = self.providers.get(provider_name).ok_or_else(|| {
            AuthError::InvalidConfig(format!("OAuth2 provider '{}' not found", provider_name))
        })?;
        provider.validate_token(access_token).await
    }
}

impl Default for OAuth2Manager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth2_token_expiry() {
        let token = OAuth2Token {
            access_token: "test_token".to_string(),
            refresh_token: None,
            expires_at: Some(Utc::now() - Duration::seconds(1)),
            token_type: "Bearer".to_string(),
            scopes: vec!["read".to_string()],
            user_id: None,
        };

        assert!(token.is_expired());

        let token_valid = OAuth2Token {
            access_token: "test_token".to_string(),
            refresh_token: None,
            expires_at: Some(Utc::now() + Duration::hours(1)),
            token_type: "Bearer".to_string(),
            scopes: vec!["read".to_string()],
            user_id: None,
        };

        assert!(!token_valid.is_expired());
    }
}
