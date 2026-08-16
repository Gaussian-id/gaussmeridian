//! Tests for the core router functionality

use crate::{
    load_balancer::{LoadBalancer, RoundRobinLoadBalancer},
    model_registry::ModelRegistry,
};

#[tokio::test]
async fn test_round_robin_load_balancer() {
    let balancer = RoundRobinLoadBalancer::new();
    let providers = vec![
        "provider1".to_string(),
        "provider2".to_string(),
        "provider3".to_string(),
    ];

    let selected1 = balancer.select_provider(&providers).await;
    let selected2 = balancer.select_provider(&providers).await;
    let selected3 = balancer.select_provider(&providers).await;
    let selected4 = balancer.select_provider(&providers).await;

    assert!(selected1.is_some());
    assert!(selected2.is_some());
    assert!(selected3.is_some());
    assert!(selected4.is_some());

    // Should cycle through providers
    assert_eq!(selected1.unwrap(), "provider1");
    assert_eq!(selected2.unwrap(), "provider2");
    assert_eq!(selected3.unwrap(), "provider3");
    assert_eq!(selected4.unwrap(), "provider1");
}

#[tokio::test]
async fn test_model_registry() {
    let registry = ModelRegistry::new();

    // Test empty registry
    assert_eq!(registry.list_all_models().await.len(), 0);
    assert!(registry
        .get_provider_for_model("gpt-4")
        .await
        .is_none());

    // Test registering models
    let models = vec![
        gaussmeridian_models::Model {
            id: "gpt-4".to_string(),
            object: "model".to_string(),
            created: 1677610602,
            owned_by: "openai".to_string(),
            permission: None,
            root: None,
            parent: None,
        },
        gaussmeridian_models::Model {
            id: "gpt-3.5-turbo".to_string(),
            object: "model".to_string(),
            created: 1677610602,
            owned_by: "openai".to_string(),
            permission: None,
            root: None,
            parent: None,
        },
    ];

    registry.register_provider_models("openai", models).await;

    assert_eq!(registry.list_all_models().await.len(), 2);
    assert_eq!(
        registry
            .get_provider_for_model("gpt-4")
            .await
            .unwrap(),
        "openai"
    );
    assert_eq!(
        registry
            .get_provider_for_model("gpt-3.5-turbo")
            .await
            .unwrap(),
        "openai"
    );
}
