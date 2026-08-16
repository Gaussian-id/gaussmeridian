use super::*;
use crate::{
    agents::{Agent, AgentResponse, MockAgent},
    error::MoaResult,
    models::MoaRequest,
};
use chrono::Utc;
use mockall::predicate::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_attention_strategy() -> MoaResult<()> {
    let strategy = AttentionStrategy::new(
        4,   // num_heads
        128, // embedding_dim
        0.01, // learning_rate
        0.1,  // temperature
        100,  // max_history
    );

    // Create mock agents
    let mut agents = Vec::new();
    for i in 0..3 {
        let mut mock = MockAgent::new();
        mock.expect_id()
            .returning(move || format!("agent_{}", i));
        mock.expect_generate_response()
            .returning(|req| {
                Ok(AgentResponse {
                    content: format!("Response from agent {}", i),
                    confidence: 0.8,
                    request: req.clone(),
                    metadata: Default::default(),
                    metrics: Default::default(),
                })
            });
        agents.push(Box::new(mock) as Box<dyn Agent>);
    }

    // Create test request
    let request = MoaRequest {
        id: Uuid::new_v4(),
        query: "Test query".to_string(),
        context: None,
        metadata: Default::default(),
        parameters: Default::default(),
        agent_id: "test".to_string(),
    };

    // Generate responses
    let mut responses = Vec::new();
    for agent in &agents {
        responses.push(agent.generate_response(&request).await?);
    }

    // Process responses
    let result = strategy.process(&agents, &responses).await?;

    assert!(result.confidence > 0.0);
    assert!(!result.content.is_empty());
    assert_eq!(result.agent_responses.len(), responses.len());

    Ok(())
}

#[tokio::test]
async fn test_clustering_strategy() -> MoaResult<()> {
    let strategy = ClusteringMoaStrategy::new(
        2,   // min_clusters
        4,   // max_clusters
        100, // max_iter
        0.5, // min_confidence
        Linkage::Average,
    );

    // Create diverse responses
    let responses = vec![
        AgentResponse {
            content: "Response about topic A".to_string(),
            confidence: 0.9,
            request: create_test_request(),
            metadata: Default::default(),
            metrics: Default::default(),
        },
        AgentResponse {
            content: "Another response about A".to_string(),
            confidence: 0.8,
            request: create_test_request(),
            metadata: Default::default(),
            metrics: Default::default(),
        },
        AgentResponse {
            content: "Response about topic B".to_string(),
            confidence: 0.7,
            request: create_test_request(),
            metadata: Default::default(),
            metrics: Default::default(),
        },
    ];

    let result = strategy.process(&Vec::new(), &responses).await?;

    // Should form at least 2 clusters
    assert!(result.content.contains("Cluster 1"));
    assert!(result.content.contains("Cluster 2"));
    assert!(result.confidence > 0.0);

    Ok(())
}

#[tokio::test]
async fn test_routing_strategy() -> MoaResult<()> {
    let strategy = RoutingStrategy::new(100, 3600, 10.0);

    // Initialize A/B test
    strategy.create_ab_test(
        "test".to_string(),
        vec!["A".to_string(), "B".to_string()],
        vec![0.5, 0.5],
        3600,
    ).await?;

    // Initialize bandit
    strategy.init_bandit(
        "test_bandit".to_string(),
        BanditAlgorithm::UCB {
            counts: Default::default(),
            values: Default::default(),
            total_pulls: 0,
            exploration_factor: 1.0,
        },
    ).await?;

    // Test agent selection
    let agents = vec!["agent_1".to_string(), "agent_2".to_string()];
    let selected = strategy.select_agent_bandit(&agents, "test_bandit").await;
    assert!(selected.is_some());

    // Record performance
    strategy.record_ab_test_metrics(
        "test",
        "A",
        100,
        true,
        0.9,
    ).await?;

    // Get results
    let results = strategy.get_ab_test_results("test").await?;
    assert!(results.contains_key("A"));
    assert!(results["A"].success_rate > 0.0);

    Ok(())
}

#[tokio::test]
async fn test_multi_head_attention() -> MoaResult<()> {
    let strategy = AttentionStrategy::new(
        2,   // num_heads
        64,  // embedding_dim
        0.01, // learning_rate
        0.1,  // temperature
        100,  // max_history
    );

    // Test attention score computation
    let query = Array1::from_vec(vec![0.1; 64]);
    let agents = create_test_agents(3);
    
    for agent in &agents {
        strategy.update_agent_embedding(
            agent.id(),
            Array1::from_vec(vec![0.1; 64]),
        ).await?;
    }

    let scores = strategy.compute_attention_scores(&query, &agents).await;
    assert_eq!(scores.len(), agents.len());
    assert!(scores.iter().all(|&s| s >= 0.0 && s <= 1.0));

    Ok(())
}

#[tokio::test]
async fn test_hierarchical_clustering() -> MoaResult<()> {
    let strategy = ClusteringMoaStrategy::new(
        2,   // min_clusters
        4,   // max_clusters
        100, // max_iter
        0.5, // min_confidence
        Linkage::Average,
    );

    // Create test features
    let features = Array2::from_shape_vec(
        (4, 4),
        vec![
            1.0, 0.1, 0.1, 0.1,
            0.1, 1.0, 0.1, 0.1,
            0.1, 0.1, 1.0, 0.8,
            0.1, 0.1, 0.8, 1.0,
        ],
    ).unwrap();

    let clusters = strategy.hierarchical_cluster(&features, 2);
    assert_eq!(clusters.len(), 4);
    
    // Points 0 and 1 should be in different clusters
    assert_ne!(clusters[0], clusters[1]);
    // Points 2 and 3 should be in the same cluster
    assert_eq!(clusters[2], clusters[3]);

    Ok(())
}

#[tokio::test]
async fn test_bandit_algorithms() -> MoaResult<()> {
    let strategy = RoutingStrategy::new(100, 3600, 10.0);

    // Test UCB
    strategy.init_bandit(
        "ucb".to_string(),
        BanditAlgorithm::UCB {
            counts: Default::default(),
            values: Default::default(),
            total_pulls: 0,
            exploration_factor: 1.0,
        },
    ).await?;

    // Test Thompson Sampling
    strategy.init_bandit(
        "thompson".to_string(),
        BanditAlgorithm::ThompsonSampling {
            successes: Default::default(),
            failures: Default::default(),
        },
    ).await?;

    // Test Epsilon Greedy
    strategy.init_bandit(
        "epsilon".to_string(),
        BanditAlgorithm::EpsilonGreedy {
            values: Default::default(),
            counts: Default::default(),
            epsilon: 0.1,
        },
    ).await?;

    let agents = vec!["A".to_string(), "B".to_string()];

    // Test each algorithm
    for name in ["ucb", "thompson", "epsilon"] {
        let selected = strategy.select_agent_bandit(&agents, name).await;
        assert!(selected.is_some());
        assert!(agents.contains(&selected.unwrap()));

        // Update with rewards
        strategy.update_bandit(name, "A", 1.0, true).await?;
        strategy.update_bandit(name, "B", 0.0, false).await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_role_based_strategy() -> MoaResult<()> {
    let strategy = RoleStrategy::new(100);
    let agents = create_test_agents(4);

    // Assign roles
    strategy.assign_role(
        "agent_0",
        AgentRole {
            name: "Math Expert".to_string(),
            description: "Specializes in mathematics".to_string(),
            specializations: vec!["math".to_string(), "calculation".to_string()],
            confidence_threshold: 0.7,
        },
    ).await?;

    strategy.assign_role(
        "agent_1",
        AgentRole {
            name: "Code Expert".to_string(),
            description: "Specializes in programming".to_string(),
            specializations: vec!["code".to_string(), "programming".to_string()],
            confidence_threshold: 0.7,
        },
    ).await?;

    // Create test requests
    let math_request = MoaRequest {
        id: Uuid::new_v4(),
        query: "Solve this math problem".to_string(),
        context: None,
        metadata: Default::default(),
        parameters: Default::default(),
        agent_id: "test".to_string(),
    };

    let code_request = MoaRequest {
        id: Uuid::new_v4(),
        query: "Help with programming".to_string(),
        context: None,
        metadata: Default::default(),
        parameters: Default::default(),
        agent_id: "test".to_string(),
    };

    // Test agent selection
    let math_selected = strategy.select_agents(&math_request, &agents, 2).await?;
    assert!(math_selected.contains(&0)); // Math expert should be selected

    let code_selected = strategy.select_agents(&code_request, &agents, 2).await?;
    assert!(code_selected.contains(&1)); // Code expert should be selected

    // Test role metrics
    strategy.record_performance("agent_0", 0.9, 0.85, 100).await?;
    let metrics = strategy.get_role_metrics("agent_0").await.unwrap();
    assert!(metrics.avg_confidence > 0.0);
    assert!(metrics.avg_quality_score > 0.0);

    Ok(())
}

#[tokio::test]
async fn test_debate_strategy() -> MoaResult<()> {
    let strategy = DebateStrategy::new(3, 0.5, 2);
    let agents = create_test_agents(4);
    let request = create_test_request();

    // Start debate
    let debate_id = Uuid::new_v4().to_string();
    let round_1_responses = strategy.start_round(&debate_id, &request, &agents).await?;
    assert!(!round_1_responses.is_empty());

    // Check debate history
    let history = strategy.get_debate_history(&debate_id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert!(history[0].moderation_feedback.is_some());

    // Test moderation feedback
    let feedback = &history[0].moderation_feedback.as_ref().unwrap();
    assert!(!feedback.quality_scores.is_empty());
    assert!(!feedback.relevance_scores.is_empty());
    assert!(!feedback.selected_indices.is_empty());

    // Test metrics
    assert!(history[0].metrics.avg_confidence > 0.0);
    assert!(history[0].metrics.avg_quality > 0.0);
    assert!(history[0].metrics.avg_relevance > 0.0);

    Ok(())
}

#[tokio::test]
async fn test_token_metrics_tracking() -> MoaResult<()> {
    let tracker = TokenMetricsTracker::new();
    let request = create_test_request();
    let agents = create_test_agents(2);

    // Track token usage for multiple categories
    for (i, agent) in agents.iter().enumerate() {
        let response = agent.generate_response(&request).await?;
        
        // Track role-based usage
        tracker.track_agent_response(
            agent.id(),
            &request,
            &response,
            100 + i as u64,
            200 + i as u64,
            "role_based",
        ).await?;

        // Track debate usage
        tracker.track_agent_response(
            agent.id(),
            &request,
            &response,
            150 + i as u64,
            250 + i as u64,
            "debate",
        ).await?;
    }

    // Update request duration
    tracker.update_request_duration(&request.id.to_string(), 1000).await?;

    // Check agent metrics
    let agent_metrics = tracker.get_agent_metrics("agent_0").await.unwrap();
    assert_eq!(agent_metrics.input_tokens, 250); // 100 + 150
    assert_eq!(agent_metrics.output_tokens, 450); // 200 + 250
    assert_eq!(agent_metrics.total_requests, 2);

    // Check request metrics
    let request_metrics = tracker.get_request_metrics(&request.id.to_string()).await.unwrap();
    assert!(request_metrics.total_tokens > 0);
    assert_eq!(request_metrics.duration_ms, 1000);
    assert!(request_metrics.token_rate > 0.0);

    // Check category breakdown
    let report = tracker.generate_report().await;
    assert!(report.category_breakdown.contains_key("role_based"));
    assert!(report.category_breakdown.contains_key("debate"));

    // Check model breakdown
    assert!(!report.model_breakdown.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_combined_strategies() -> MoaResult<()> {
    let role_strategy = RoleStrategy::new(100);
    let debate_strategy = DebateStrategy::new(3, 0.5, 2);
    let attention_strategy = AttentionStrategy::new(4, 128, 0.01, 0.1, 100);
    let token_tracker = TokenMetricsTracker::new();

    let agents = create_test_agents(4);
    let request = create_test_request();

    // 1. Select agents using role strategy
    let selected_indices = role_strategy.select_agents(&request, &agents, 2).await?;
    let selected_agents: Vec<_> = selected_indices.iter().map(|&i| &agents[i]).collect();
    assert_eq!(selected_agents.len(), 2);

    // 2. Run debate with selected agents
    let debate_id = Uuid::new_v4().to_string();
    let debate_responses = debate_strategy.start_round(&debate_id, &request, &selected_agents).await?;
    assert!(!debate_responses.is_empty());

    // Track token usage
    for (agent, response) in selected_agents.iter().zip(debate_responses.iter()) {
        token_tracker.track_agent_response(
            agent.id(),
            &request,
            response,
            200,
            300,
            "combined_strategy",
        ).await?;
    }

    // 3. Use attention for final aggregation
    let final_result = attention_strategy.process(&agents, &debate_responses).await?;
    assert!(final_result.confidence > 0.0);
    assert!(!final_result.content.is_empty());

    // Check metrics
    let report = token_tracker.generate_report().await;
    assert!(report.category_breakdown.contains_key("combined_strategy"));

    Ok(())
}

#[tokio::test]
async fn test_reconcile_strategy() -> MoaResult<()> {
    let strategy = ReConcileStrategy::new(3, 0.5, 0.8);
    let agents = create_test_agents(4);
    let request = create_test_request();

    // Start reconciliation
    let reconciliation_id = Uuid::new_v4().to_string();
    let round_1_responses = strategy.start_round(&reconciliation_id, &request, &agents).await?;
    assert!(!round_1_responses.is_empty());

    // Check history
    let history = strategy.get_history(&reconciliation_id).await.unwrap();
    assert_eq!(history.len(), 1);

    // Check round metrics
    let round = &history[0];
    assert!(round.metrics.agreement_level >= 0.0);
    assert!(round.metrics.resolution_rate >= 0.0);
    assert!(round.metrics.duration_ms > 0);

    // Process responses
    let result = strategy.process(&agents, &round_1_responses).await?;
    assert!(result.confidence > 0.0);
    assert!(!result.content.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_reconcile_conflict_resolution() -> MoaResult<()> {
    let strategy = ReConcileStrategy::new(3, 0.5, 0.8);
    let mut agents = create_test_agents(3);

    // Create conflicting responses
    let request = MoaRequest {
        id: Uuid::new_v4(),
        query: "What is 2 + 2?".to_string(),
        context: None,
        metadata: Default::default(),
        parameters: Default::default(),
        agent_id: "test".to_string(),
    };

    // Mock agents with conflicting responses
    let mut mock1 = MockAgent::new();
    mock1.expect_id().returning(|| "agent_1".to_string());
    mock1.expect_generate_response()
        .returning(|req| {
            Ok(AgentResponse {
                content: "2 + 2 = 4".to_string(),
                confidence: 0.9,
                request: req.clone(),
                metadata: Default::default(),
                metrics: Default::default(),
            })
        });

    let mut mock2 = MockAgent::new();
    mock2.expect_id().returning(|| "agent_2".to_string());
    mock2.expect_generate_response()
        .returning(|req| {
            Ok(AgentResponse {
                content: "2 + 2 = 5".to_string(), // Intentionally wrong
                confidence: 0.7,
                request: req.clone(),
                metadata: Default::default(),
                metrics: Default::default(),
            })
        });

    agents = vec![Box::new(mock1), Box::new(mock2)];

    // Start reconciliation
    let reconciliation_id = Uuid::new_v4().to_string();
    let responses = strategy.start_round(&reconciliation_id, &request, &agents).await?;

    // Check history
    let history = strategy.get_history(&reconciliation_id).await.unwrap();
    let round = &history[0];

    // Verify conflicts were identified
    assert!(!round.conflicts.is_empty());
    
    // Check conflict type
    let conflict = &round.conflicts[0];
    assert!(matches!(conflict.conflict_type, ConflictType::Factual));

    // Verify resolution attempts
    assert!(!round.resolutions.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_reconcile_knowledge_base() -> MoaResult<()> {
    let strategy = ReConcileStrategy::new(3, 0.5, 0.8);
    let agents = create_test_agents(3);

    // Create a sequence of related requests
    let requests = vec![
        MoaRequest {
            id: Uuid::new_v4(),
            query: "What is Python?".to_string(),
            context: None,
            metadata: Default::default(),
            parameters: Default::default(),
            agent_id: "test".to_string(),
        },
        MoaRequest {
            id: Uuid::new_v4(),
            query: "Compare Python and Java".to_string(),
            context: None,
            metadata: Default::default(),
            parameters: Default::default(),
            agent_id: "test".to_string(),
        },
    ];

    // Process multiple rounds to build knowledge
    for request in &requests {
        let reconciliation_id = Uuid::new_v4().to_string();
        let responses = strategy.start_round(&reconciliation_id, request, &agents).await?;
        
        // Process responses
        let result = strategy.process(&agents, &responses).await?;
        assert!(result.confidence > 0.0);
    }

    // Reset strategy
    strategy.reset().await?;

    Ok(())
}

#[tokio::test]
async fn test_reconcile_consensus_building() -> MoaResult<()> {
    let strategy = ReConcileStrategy::new(5, 0.5, 0.8);
    let agents = create_test_agents(4);
    let request = create_test_request();

    let reconciliation_id = Uuid::new_v4().to_string();
    let mut consensus_reached = false;
    let mut round = 0;

    while round < 5 && !consensus_reached {
        let responses = strategy.start_round(&reconciliation_id, &request, &agents).await?;
        
        // Check history
        if let Some(history) = strategy.get_history(&reconciliation_id).await {
            if let Some(last_round) = history.last() {
                consensus_reached = last_round.consensus_reached;
                if consensus_reached {
                    // Verify consensus metrics
                    assert!(last_round.metrics.agreement_level >= 0.8);
                    break;
                }
            }
        }

        round += 1;
    }

    Ok(())
}

// Helper functions
fn create_test_request() -> MoaRequest {
    MoaRequest {
        id: Uuid::new_v4(),
        query: "Test query".to_string(),
        context: None,
        metadata: Default::default(),
        parameters: Default::default(),
        agent_id: "test".to_string(),
    }
}

fn create_test_agents(count: usize) -> Vec<Box<dyn Agent>> {
    let mut agents = Vec::new();
    for i in 0..count {
        let mut mock = MockAgent::new();
        mock.expect_id()
            .returning(move || format!("agent_{}", i));
        agents.push(Box::new(mock) as Box<dyn Agent>);
    }
    agents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::MockAgent;
    use crate::models::MoaRequest;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_empty_responses() {
        let strategy = ReConcileStrategy::new_with_defaults(3, 0.5, 0.7);
        let agents: Vec<Box<dyn Agent>> = vec![];
        let responses: Vec<AgentResponse> = vec![];
        
        let result = strategy.process(&agents, &responses).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MoaError::Strategy(_)));
    }

    #[tokio::test]
    async fn test_single_agent_response() {
        let strategy = ReConcileStrategy::new_with_defaults(3, 0.5, 0.7);
        let mut mock_agent = MockAgent::new();
        mock_agent.expect_id().returning(|| "agent1".to_string());
        
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(mock_agent)];
        let responses = vec![AgentResponse {
            content: "Test response".into(),
            confidence: 0.8,
            agent_id: "agent1".into(),
            request: MoaRequest { query: "test".into() },
        }];
        
        let result = strategy.process(&agents, &responses).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_conflicting_numerical_responses() {
        let strategy = ReConcileStrategy::new_with_defaults(3, 0.5, 0.7);
        let mut mock_agent1 = MockAgent::new();
        let mut mock_agent2 = MockAgent::new();
        
        mock_agent1.expect_id().returning(|| "agent1".to_string());
        mock_agent2.expect_id().returning(|| "agent2".to_string());
        
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(mock_agent1), Box::new(mock_agent2)];
        let responses = vec![
            AgentResponse {
                content: "The temperature is 25.0 degrees".into(),
                confidence: 0.9,
                agent_id: "agent1".into(),
                request: MoaRequest { query: "temperature".into() },
            },
            AgentResponse {
                content: "The temperature is 30.0 degrees".into(),
                confidence: 0.85,
                agent_id: "agent2".into(),
                request: MoaRequest { query: "temperature".into() },
            },
        ];
        
        let result = strategy.process(&agents, &responses).await;
        assert!(result.is_ok());
        let conflicts = strategy.identify_conflicts(&responses).await.unwrap();
        assert!(!conflicts.is_empty());
        assert_eq!(conflicts[0].conflict_type, ConflictType::Factual);
    }

    #[tokio::test]
    async fn test_perspective_conflict_resolution() {
        let strategy = ReConcileStrategy::new_with_defaults(3, 0.5, 0.7);
        let responses = vec![
            AgentResponse {
                content: "The code is well-structured but could use more comments".into(),
                confidence: 0.8,
                agent_id: "agent1".into(),
                request: MoaRequest { query: "code review".into() },
            },
            AgentResponse {
                content: "The code has good documentation but needs refactoring".into(),
                confidence: 0.85,
                agent_id: "agent2".into(),
                request: MoaRequest { query: "code review".into() },
            },
        ];
        
        let conflicts = strategy.identify_conflicts(&responses).await.unwrap();
        assert!(!conflicts.is_empty());
        
        let resolutions = strategy.resolve_conflicts(&conflicts, &responses).await.unwrap();
        assert!(!resolutions.is_empty());
        assert!(resolutions[0].description.contains("perspective"));
    }

    #[tokio::test]
    async fn test_batch_processing_edge_cases() {
        let strategy = ReConcileStrategy::new_with_defaults(3, 0.5, 0.7);
        let requests = vec![
            MoaRequest { query: "test1".into() },
            MoaRequest { query: "test2".into() },
            MoaRequest { query: "test3".into() },
        ];
        
        // Test with no agents
        let empty_agents: Vec<Box<dyn Agent>> = vec![];
        let result = strategy.process_batch(&requests, &empty_agents, 2).await;
        assert!(result.is_err());

        // Test with batch size larger than requests
        let mut mock_agent = MockAgent::new();
        mock_agent.expect_id().returning(|| "agent1".to_string());
        mock_agent.expect_generate_response()
            .returning(|_| Ok(AgentResponse {
                content: "Test response".into(),
                confidence: 0.8,
                agent_id: "agent1".into(),
                request: MoaRequest { query: "test".into() },
            }));
        
        let agents = vec![Box::new(mock_agent)];
        let result = strategy.process_batch(&requests, &agents, 5).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_knowledge_base_access() {
        let strategy = ReConcileStrategy::new_with_defaults(3, 0.5, 0.7);
        let mut handles = Vec::new();
        
        for i in 0..10 {
            let strategy = strategy.clone();
            handles.push(tokio::spawn(async move {
                let mock_response = AgentResponse {
                    content: format!("Test response {}", i),
                    confidence: 0.8,
                    agent_id: format!("agent{}", i),
                    request: MoaRequest { query: "test".into() },
                };
                
                strategy.update_knowledge_base(&[mock_response], &[]).await
            }));
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_resource_optimization() {
        let strategy = ReConcileStrategy::new_with_defaults(3, 0.5, 0.7);
        let mut mock_agent = MockAgent::new();
        mock_agent.expect_id().returning(|| "agent1".to_string());
        mock_agent.expect_generate_response()
            .returning(|_| {
                sleep(Duration::from_millis(100)).await;
                Ok(AgentResponse {
                    content: "Test response".into(),
                    confidence: 0.8,
                    agent_id: "agent1".into(),
                    request: MoaRequest { query: "test".into() },
                })
            });

        let agents = vec![Box::new(mock_agent)];
        let requests = (0..5).map(|i| MoaRequest { query: format!("test{}", i) }).collect::<Vec<_>>();
        
        let (responses, metrics) = strategy.optimize_batch(&requests, &agents).await.unwrap();
        
        assert_eq!(responses.len(), requests.len());
        assert!(metrics.total_time_ms > 0);
        assert!(metrics.avg_response_time_ms > 0);
        assert!(metrics.batch_size > 0);
    }

    #[tokio::test]
    async fn test_consensus_patterns() {
        let strategy = ReConcileStrategy::new_with_defaults(3, 0.5, 0.7);
        let responses = vec![
            AgentResponse {
                content: "Critical system status: OK".into(),
                confidence: 0.95,
                agent_id: "agent1".into(),
                request: MoaRequest { query: "system status".into() },
            },
            AgentResponse {
                content: "Critical system status: OK".into(),
                confidence: 0.92,
                agent_id: "agent2".into(),
                request: MoaRequest { query: "system status".into() },
            },
        ];

        let agreement = strategy.calculate_agreement_level(&responses).await;
        assert!(agreement > 0.9);
    }

    #[tokio::test]
    async fn test_knowledge_base_updates() {
        let strategy = ReConcileStrategy::new_with_defaults(3, 0.5, 0.7);
        let response = AgentResponse {
            content: "New programming best practice: Use type hints".into(),
            confidence: 0.9,
            agent_id: "expert_agent".into(),
            request: MoaRequest { query: "programming best practices".into() },
        };

        strategy.update_knowledge_base(&[response], &[]).await.unwrap();
        
        let kb = strategy.knowledge_base.read().await;
        let technical_entries = kb.domain_knowledge.get("technical").unwrap();
        assert!(!technical_entries.is_empty());
    }
} 