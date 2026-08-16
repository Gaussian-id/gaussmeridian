//! Integration test framework for GaussMeridian
//!
//! This module provides common utilities and fixtures for integration testing.

use gaussmeridian_db::{client::DatabaseClient, error::DatabaseError};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Test database configuration
pub struct TestDatabase {
    pub client: Arc<DatabaseClient>,
    pub namespace: String,
    pub database: String,
}

static TEST_DB: OnceCell<Arc<TestDatabase>> = OnceCell::const_new();

impl TestDatabase {
    /// Get or create a test database instance
    pub async fn get() -> Arc<TestDatabase> {
        TEST_DB
            .get_or_init(|| async {
                let namespace = format!("test_{}", uuid::Uuid::new_v4());
                let database = "gaussmeridian_test";

                let client = DatabaseClient::new(
                    "ws://localhost:8000",
                    &namespace,
                    database,
                    "root",
                    "root",
                )
                .await
                .expect("Failed to connect to test database");

                Arc::new(TestDatabase {
                    client: Arc::new(client),
                    namespace,
                    database: database.to_string(),
                })
            })
            .await
            .clone()
    }

    /// Clean up all test data
    pub async fn cleanup(&self) -> Result<(), DatabaseError> {
        // Delete all collections
        self.client.query("DELETE FROM users").await?;
        self.client.query("DELETE FROM api_keys").await?;
        self.client.query("DELETE FROM tenants").await?;
        self.client.query("DELETE FROM requests").await?;
        self.client.query("DELETE FROM responses").await?;
        self.client.query("DELETE FROM rate_limits").await?;
        Ok(())
    }
}

/// Test fixture for creating test users
pub struct UserFixture;

impl UserFixture {
    pub async fn create_test_user(
        db: &DatabaseClient,
        email: &str,
        username: &str,
        password_hash: &str,
    ) -> Result<String, DatabaseError> {
        let user = gaussmeridian_db::schema::User {
            id: None,
            email: email.to_string(),
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            tenant_id: None,
            roles: vec!["developer".to_string()],
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

        let repo = gaussmeridian_db::user_repository::UserRepository::new(db.clone());
        repo.create(user).await
    }

    pub async fn create_admin_user(
        db: &DatabaseClient,
        email: &str,
        username: &str,
        password_hash: &str,
    ) -> Result<String, DatabaseError> {
        let user = gaussmeridian_db::schema::User {
            id: None,
            email: email.to_string(),
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            tenant_id: None,
            roles: vec!["admin".to_string(), "developer".to_string()],
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

        let repo = gaussmeridian_db::user_repository::UserRepository::new(db.clone());
        repo.create(user).await
    }
}

/// Test fixture for creating API keys
pub struct ApiKeyFixture;

impl ApiKeyFixture {
    pub async fn create_test_api_key(
        db: &DatabaseClient,
        user_id: &str,
        key_hash: &str,
        key_prefix: &str,
    ) -> Result<String, DatabaseError> {
        let api_key = gaussmeridian_db::schema::ApiKey {
            id: None,
            key_hash: key_hash.to_string(),
            key_prefix: key_prefix.to_string(),
            user_id: user_id.to_string(),
            tenant_id: None,
            project_id: None,
            name: Some("Test API Key".to_string()),
            rate_limit_per_minute: Some(100),
            rate_limit_per_day: Some(10000),
            created_at: chrono::Utc::now(),
            expires_at: None,
            last_used_at: None,
            active: true,
        };

        let repo = gaussmeridian_db::api_key_repository::ApiKeyRepository::new(db.clone());
        repo.create(api_key).await
    }
}

/// Test fixture for creating tenants
pub struct TenantFixture;

impl TenantFixture {
    pub async fn create_test_tenant(
        db: &DatabaseClient,
        name: &str,
    ) -> Result<String, DatabaseError> {
        let tenant = gaussmeridian_db::schema::Tenant {
            id: None,
            name: name.to_string(),
            api_key_prefix: format!("{}_", name),
            rate_limit_per_minute: Some(1000),
            rate_limit_per_day: Some(100000),
            max_users: Some(10),
            features: vec!["chat".to_string(), "embeddings".to_string()],
            metadata: json!({"plan": "enterprise"}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            active: true,
        };

        let repo = gaussmeridian_db::tenant_repository::TenantRepository::new(db.clone());
        repo.create(tenant).await
    }
}

/// Test utilities
pub mod utils {
    use sha2::{Digest, Sha256};

    /// Hash a password for testing
    pub fn hash_password(password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Hash an API key for testing
    pub fn hash_api_key(api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generate a random API key for testing
    pub fn generate_test_api_key() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let key_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        key_bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_connection() {
        let db = TestDatabase::get().await;
        assert!(db.client.is_some());
    }

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash1 = utils::hash_password(password);
        let hash2 = utils::hash_password(password);
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, password);
    }

    #[test]
    fn test_api_key_generation() {
        let key1 = utils::generate_test_api_key();
        let key2 = utils::generate_test_api_key();
        assert_ne!(key1, key2);
        assert_eq!(key1.len(), 64); // 32 bytes = 64 hex chars
    }
}

