//! End-to-end integration tests for GaussMeridian
//!
//! Tests critical user flows from registration to API usage.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{TestDatabase, UserFixture};
use serde_json::json;
use tower::ServiceExt;

/// Test the complete user registration and authentication flow
#[tokio::test]
async fn test_e2e_user_registration_and_login() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    // This test would require the full server setup
    // For now, we'll test the components individually
    
    // 1. Create user via AuthManager
    let password_hash = common::utils::hash_password("SecurePass123!");
    let user_id = UserFixture::create_test_user(
        &db.client,
        "e2e_user@example.com",
        "e2e_user",
        &password_hash,
    )
    .await
    .unwrap();

    // Verify user was created
    let repo = gaussmeridian_db::user_repository::UserRepository::new((*db.client).clone());
    let user = repo.get_by_email("e2e_user@example.com").await.unwrap();
    assert!(user.is_some());
    assert_eq!(user.unwrap().username, "e2e_user");
}

/// Test API key creation and usage flow
#[tokio::test]
async fn test_e2e_api_key_lifecycle() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    // 1. Create user
    let password_hash = common::utils::hash_password("SecurePass123!");
    let user_id = UserFixture::create_test_user(
        &db.client,
        "api_user@example.com",
        "api_user",
        &password_hash,
    )
    .await
    .unwrap();

    // 2. Create API key
    let api_key = common::utils::generate_test_api_key();
    let key_hash = common::utils::hash_api_key(&api_key);
    let key_prefix = api_key.chars().take(8).collect::<String>();

    let key_id =
        common::ApiKeyFixture::create_test_api_key(&db.client, &user_id, &key_hash, &key_prefix)
            .await
            .unwrap();

    // 3. Validate API key exists
    let repo = gaussmeridian_db::api_key_repository::ApiKeyRepository::new((*db.client).clone());
    let stored_key = repo.get_by_key_hash(&key_hash).await.unwrap();
    assert!(stored_key.is_some());
    assert_eq!(stored_key.unwrap().user_id, user_id);

    // 4. Revoke API key (deactivate)
    let mut key = repo.get_by_id(&key_id).await.unwrap().unwrap();
    key.active = false;
    repo.update(&key_id, key).await.unwrap();

    // 5. Verify key is inactive
    let inactive_key = repo.get_by_id(&key_id).await.unwrap().unwrap();
    assert!(!inactive_key.active);
}

/// Test rate limiting enforcement
#[tokio::test]
async fn test_e2e_rate_limiting() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    use gaussmeridian_core::{DistributedRateLimiter, DistributedRateLimiterConfig};

    // Create rate limiter with low limits for testing
    let config = DistributedRateLimiterConfig {
        requests_per_minute: 5,
        tokens_per_minute: 500,
        window_duration_secs: 60,
    };

    let limiter = DistributedRateLimiter::new((*db.client).clone(), config);

    // Make requests up to limit
    for i in 0..5 {
        let result = limiter.check_rate_limit("test_user", 50).await.unwrap();
        assert!(result.allowed, "Request {} should be allowed", i + 1);
    }

    // Next request should be rate limited
    let result = limiter.check_rate_limit("test_user", 50).await.unwrap();
    assert!(
        !result.allowed,
        "Request 6 should be rate limited (max 5)"
    );
    assert_eq!(result.remaining_requests, 0);
}

/// Test token-based rate limiting
#[tokio::test]
async fn test_e2e_token_rate_limiting() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    use gaussmeridian_core::{DistributedRateLimiter, DistributedRateLimiterConfig};

    let config = DistributedRateLimiterConfig {
        requests_per_minute: 100,
        tokens_per_minute: 1000,
        window_duration_secs: 60,
    };

    let limiter = DistributedRateLimiter::new((*db.client).clone(), config);

    // Make request with 600 tokens
    let result = limiter
        .check_rate_limit("token_test_user", 600)
        .await
        .unwrap();
    assert!(result.allowed);
    assert_eq!(result.remaining_tokens, 400);

    // Make another request with 500 tokens (should exceed limit)
    let result = limiter
        .check_rate_limit("token_test_user", 500)
        .await
        .unwrap();
    assert!(!result.allowed, "Should be rate limited by token count");
}

/// Test usage tracking and cost calculation
#[tokio::test]
async fn test_e2e_usage_tracking() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    use gaussmeridian_core::{RequestUsage, UsageTracker};

    let tracker = UsageTracker::new((*db.client).clone());

    // Track a request
    let request = RequestUsage {
        request_id: uuid::Uuid::new_v4().to_string(),
        user_id: Some("usage_test_user".to_string()),
        api_key_id: Some("test_key_123".to_string()),
        tenant_id: None,
        model: "gpt-4".to_string(),
        provider: "openai".to_string(),
        endpoint: "chat.completions".to_string(),
        prompt_tokens: Some(100),
        completion_tokens: Some(50),
        total_tokens: Some(150),
        status: "success".to_string(),
        error_message: None,
        latency_ms: Some(250),
    };

    let request_id = tracker.track_request(request.clone()).await.unwrap();

    // Verify request was tracked
    let stored_request = tracker.get_request_usage(&request.request_id).await.unwrap();
    assert!(stored_request.is_some());

    let stored = stored_request.unwrap();
    assert_eq!(stored.model, "gpt-4");
    assert_eq!(stored.provider, "openai");
    assert!(stored.cost.is_some(), "Cost should be calculated");

    // Cost calculation check (GPT-4: $0.03 per 1K prompt tokens, $0.06 per 1K completion tokens)
    let expected_cost = (100.0 / 1000.0) * 0.03 + (50.0 / 1000.0) * 0.06;
    let actual_cost = stored.cost.unwrap();
    assert!(
        (actual_cost - expected_cost).abs() < 0.0001,
        "Cost calculation incorrect: expected {}, got {}",
        expected_cost,
        actual_cost
    );
}

/// Test usage summary aggregation
#[tokio::test]
async fn test_e2e_usage_summary() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    use gaussmeridian_core::{RequestUsage, UsageTracker};

    let tracker = UsageTracker::new((*db.client).clone());
    let user_id = "summary_test_user";

    // Track multiple requests
    for i in 0..5 {
        let request = RequestUsage {
            request_id: uuid::Uuid::new_v4().to_string(),
            user_id: Some(user_id.to_string()),
            api_key_id: Some("test_key_123".to_string()),
            tenant_id: None,
            model: "gpt-3.5-turbo".to_string(),
            provider: "openai".to_string(),
            endpoint: "chat.completions".to_string(),
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: Some(150),
            status: "success".to_string(),
            error_message: None,
            latency_ms: Some(200),
        };

        tracker.track_request(request).await.unwrap();
    }

    // Get usage summary
    let start_date = chrono::Utc::now() - chrono::Duration::hours(1);
    let end_date = chrono::Utc::now() + chrono::Duration::hours(1);

    let summary = tracker
        .get_user_usage_summary(user_id, start_date, end_date)
        .await
        .unwrap();

    assert_eq!(summary.total_requests, 5);
    assert_eq!(summary.total_prompt_tokens, 500);
    assert_eq!(summary.total_completion_tokens, 250);
    assert_eq!(summary.total_tokens, 750);
    assert!(summary.total_cost > 0.0);
}

/// Test model-specific usage analytics
#[tokio::test]
async fn test_e2e_model_usage_analytics() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    use gaussmeridian_core::{RequestUsage, UsageTracker};

    let tracker = UsageTracker::new((*db.client).clone());
    let user_id = "analytics_test_user";

    // Track requests for different models
    let models = vec![
        ("gpt-4", "openai", 2),
        ("gpt-3.5-turbo", "openai", 3),
        ("claude-3-sonnet", "anthropic", 2),
    ];

    for (model, provider, count) in models {
        for _ in 0..count {
            let request = RequestUsage {
                request_id: uuid::Uuid::new_v4().to_string(),
                user_id: Some(user_id.to_string()),
                api_key_id: Some("test_key_123".to_string()),
                tenant_id: None,
                model: model.to_string(),
                provider: provider.to_string(),
                endpoint: "chat.completions".to_string(),
                prompt_tokens: Some(100),
                completion_tokens: Some(50),
                total_tokens: Some(150),
                status: "success".to_string(),
                error_message: None,
                latency_ms: Some(200),
            };

            tracker.track_request(request).await.unwrap();
        }
    }

    // Get model usage summary
    let start_date = chrono::Utc::now() - chrono::Duration::hours(1);
    let end_date = chrono::Utc::now() + chrono::Duration::hours(1);

    let model_summaries = tracker
        .get_model_usage_summary(user_id, start_date, end_date)
        .await
        .unwrap();

    assert!(model_summaries.len() >= 3);

    // Verify each model has correct request count
    for summary in model_summaries {
        match summary.model.as_str() {
            "gpt-4" => assert_eq!(summary.request_count, 2),
            "gpt-3.5-turbo" => assert_eq!(summary.request_count, 3),
            "claude-3-sonnet" => assert_eq!(summary.request_count, 2),
            _ => {}
        }
    }
}

/// Test multi-tenant isolation
#[tokio::test]
async fn test_e2e_tenant_isolation() {
    let db = TestDatabase::get().await;
    db.cleanup().await.unwrap();

    // Create two tenants
    let tenant1_id = common::TenantFixture::create_test_tenant(&db.client, "Tenant 1")
        .await
        .unwrap();
    let tenant2_id = common::TenantFixture::create_test_tenant(&db.client, "Tenant 2")
        .await
        .unwrap();

    // Create users for each tenant
    let repo = gaussmeridian_db::user_repository::UserRepository::new((*db.client).clone());

    let user1 = gaussmeridian_db::schema::User {
        id: None,
        email: "user1@tenant1.com".to_string(),
        username: "user1_t1".to_string(),
        password_hash: common::utils::hash_password("password"),
        tenant_id: Some(tenant1_id.clone()),
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
    repo.create(user1).await.unwrap();

    let user2 = gaussmeridian_db::schema::User {
        id: None,
        email: "user2@tenant2.com".to_string(),
        username: "user2_t2".to_string(),
        password_hash: common::utils::hash_password("password"),
        tenant_id: Some(tenant2_id.clone()),
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
    repo.create(user2).await.unwrap();

    // Verify tenant isolation
    let tenant1_users = repo.list_by_tenant(&tenant1_id).await.unwrap();
    let tenant2_users = repo.list_by_tenant(&tenant2_id).await.unwrap();

    assert_eq!(tenant1_users.len(), 1);
    assert_eq!(tenant2_users.len(), 1);
    assert_eq!(tenant1_users[0].email, "user1@tenant1.com");
    assert_eq!(tenant2_users[0].email, "user2@tenant2.com");
}

