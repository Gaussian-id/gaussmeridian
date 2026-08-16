//! Database integration tests for GaussMeridian
//!
//! Tests all repository operations with a real SurrealDB instance.

mod common;

use common::{ApiKeyFixture, TenantFixture, TestDatabase, UserFixture};
use gaussmeridian_db::{
    api_key_repository::{ApiKeyRepository, ApiKeyRepositoryTrait},
    schema::{ApiKey, User},
    tenant_repository::{TenantRepository, TenantRepositoryTrait},
    user_repository::{UserRepository, UserRepositoryTrait},
};

#[tokio::test]
async fn test_user_repository_create_and_get() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    let password_hash = common::utils::hash_password("test_password_123");
    let user_id = UserFixture::create_test_user(
        &db.client,
        "test@example.com",
        "testuser",
        &password_hash,
    )
    .await
    .unwrap();

    // Get user by ID
    let repo = UserRepository::new((*db.client).clone());
    let user = repo.get_by_id(&user_id).await.unwrap();
    assert!(user.is_some());

    let user = user.unwrap();
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.username, "testuser");
    assert_eq!(user.password_hash, password_hash);
    assert!(user.active);
}

#[tokio::test]
async fn test_user_repository_get_by_email() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    let password_hash = common::utils::hash_password("test_password_123");
    let _user_id = UserFixture::create_test_user(
        &db.client,
        "test2@example.com",
        "testuser2",
        &password_hash,
    )
    .await
    .unwrap();

    let repo = UserRepository::new((*db.client).clone());
    let user = repo.get_by_email("test2@example.com").await.unwrap();
    assert!(user.is_some());

    let user = user.unwrap();
    assert_eq!(user.username, "testuser2");
}

#[tokio::test]
async fn test_user_repository_get_by_username() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    let password_hash = common::utils::hash_password("test_password_123");
    let _user_id = UserFixture::create_test_user(
        &db.client,
        "test3@example.com",
        "testuser3",
        &password_hash,
    )
    .await
    .unwrap();

    let repo = UserRepository::new((*db.client).clone());
    let user = repo.get_by_username("testuser3").await.unwrap();
    assert!(user.is_some());

    let user = user.unwrap();
    assert_eq!(user.email, "test3@example.com");
}

#[tokio::test]
async fn test_user_repository_update() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    let password_hash = common::utils::hash_password("test_password_123");
    let user_id = UserFixture::create_test_user(
        &db.client,
        "test4@example.com",
        "testuser4",
        &password_hash,
    )
    .await
    .unwrap();

    let repo = UserRepository::new((*db.client).clone());
    let mut user = repo.get_by_id(&user_id).await.unwrap().unwrap();

    // Update user
    user.username = "updated_username".to_string();
    user.updated_at = chrono::Utc::now();

    let updated = repo.update(&user_id, user.clone()).await.unwrap();
    assert!(updated.is_some());

    let updated = updated.unwrap();
    assert_eq!(updated.username, "updated_username");
}

#[tokio::test]
async fn test_user_repository_delete() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    let password_hash = common::utils::hash_password("test_password_123");
    let user_id = UserFixture::create_test_user(
        &db.client,
        "test5@example.com",
        "testuser5",
        &password_hash,
    )
    .await
    .unwrap();

    let repo = UserRepository::new((*db.client).clone());
    let deleted = repo.delete(&user_id).await.unwrap();
    assert!(deleted);

    // Verify user is deleted
    let user = repo.get_by_id(&user_id).await.unwrap();
    assert!(user.is_none());
}

#[tokio::test]
async fn test_api_key_repository_create_and_get() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    // Create a user first
    let password_hash = common::utils::hash_password("test_password_123");
    let user_id = UserFixture::create_test_user(
        &db.client,
        "test6@example.com",
        "testuser6",
        &password_hash,
    )
    .await
    .unwrap();

    // Create API key
    let test_key = common::utils::generate_test_api_key();
    let key_hash = common::utils::hash_api_key(&test_key);
    let key_prefix = test_key.chars().take(8).collect::<String>();

    let key_id = ApiKeyFixture::create_test_api_key(&db.client, &user_id, &key_hash, &key_prefix)
        .await
        .unwrap();

    // Get API key by ID
    let repo = ApiKeyRepository::new((*db.client).clone());
    let api_key = repo.get_by_id(&key_id).await.unwrap();
    assert!(api_key.is_some());

    let api_key = api_key.unwrap();
    assert_eq!(api_key.user_id, user_id);
    assert_eq!(api_key.key_hash, key_hash);
    assert!(api_key.active);
}

#[tokio::test]
async fn test_api_key_repository_get_by_key_hash() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    // Create a user first
    let password_hash = common::utils::hash_password("test_password_123");
    let user_id = UserFixture::create_test_user(
        &db.client,
        "test7@example.com",
        "testuser7",
        &password_hash,
    )
    .await
    .unwrap();

    // Create API key
    let test_key = common::utils::generate_test_api_key();
    let key_hash = common::utils::hash_api_key(&test_key);
    let key_prefix = test_key.chars().take(8).collect::<String>();

    let _key_id =
        ApiKeyFixture::create_test_api_key(&db.client, &user_id, &key_hash, &key_prefix)
            .await
            .unwrap();

    // Get API key by hash
    let repo = ApiKeyRepository::new((*db.client).clone());
    let api_key = repo.get_by_key_hash(&key_hash).await.unwrap();
    assert!(api_key.is_some());

    let api_key = api_key.unwrap();
    assert_eq!(api_key.user_id, user_id);
}

#[tokio::test]
async fn test_api_key_repository_get_by_user_id() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    // Create a user first
    let password_hash = common::utils::hash_password("test_password_123");
    let user_id = UserFixture::create_test_user(
        &db.client,
        "test8@example.com",
        "testuser8",
        &password_hash,
    )
    .await
    .unwrap();

    // Create multiple API keys
    for i in 0..3 {
        let test_key = common::utils::generate_test_api_key();
        let key_hash = common::utils::hash_api_key(&test_key);
        let key_prefix = test_key.chars().take(8).collect::<String>();

        ApiKeyFixture::create_test_api_key(&db.client, &user_id, &key_hash, &key_prefix)
            .await
            .unwrap();
    }

    // Get all API keys for user
    let repo = ApiKeyRepository::new((*db.client).clone());
    let api_keys = repo.get_by_user_id(&user_id).await.unwrap();
    assert_eq!(api_keys.len(), 3);
}

#[tokio::test]
async fn test_api_key_repository_update_last_used() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    // Create a user first
    let password_hash = common::utils::hash_password("test_password_123");
    let user_id = UserFixture::create_test_user(
        &db.client,
        "test9@example.com",
        "testuser9",
        &password_hash,
    )
    .await
    .unwrap();

    // Create API key
    let test_key = common::utils::generate_test_api_key();
    let key_hash = common::utils::hash_api_key(&test_key);
    let key_prefix = test_key.chars().take(8).collect::<String>();

    let key_id = ApiKeyFixture::create_test_api_key(&db.client, &user_id, &key_hash, &key_prefix)
        .await
        .unwrap();

    // Update last_used
    let repo = ApiKeyRepository::new((*db.client).clone());
    repo.update_last_used(&key_id).await.unwrap();

    // Verify last_used is set
    let api_key = repo.get_by_id(&key_id).await.unwrap().unwrap();
    assert!(api_key.last_used_at.is_some());
}

#[tokio::test]
async fn test_tenant_repository_create_and_get() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    let tenant_id = TenantFixture::create_test_tenant(&db.client, "Test Tenant")
        .await
        .unwrap();

    let repo = TenantRepository::new((*db.client).clone());
    let tenant = repo.get_by_id(&tenant_id).await.unwrap();
    assert!(tenant.is_some());

    let tenant = tenant.unwrap();
    assert_eq!(tenant.name, "Test Tenant");
    assert!(tenant.active);
}

#[tokio::test]
async fn test_tenant_repository_list_all() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    // Create multiple tenants
    for i in 0..3 {
        TenantFixture::create_test_tenant(&db.client, &format!("Tenant {}", i))
            .await
            .unwrap();
    }

    let repo = TenantRepository::new((*db.client).clone());
    let tenants = repo.list_all().await.unwrap();
    assert!(tenants.len() >= 3);
}

#[tokio::test]
async fn test_user_list_by_tenant() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    // Create a tenant
    let tenant_id = TenantFixture::create_test_tenant(&db.client, "Test Tenant")
        .await
        .unwrap();

    // Create users with tenant
    let repo = UserRepository::new((*db.client).clone());
    for i in 0..3 {
        let user = User {
            id: None,
            email: format!("tenant_user{}@example.com", i),
            username: format!("tenant_user{}", i),
            password_hash: common::utils::hash_password("password"),
            tenant_id: Some(tenant_id.clone()),
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
        repo.create(user).await.unwrap();
    }

    // List users by tenant
    let users = repo.list_by_tenant(&tenant_id).await.unwrap();
    assert_eq!(users.len(), 3);
}

