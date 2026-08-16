# GaussMOA User Guide

## Table of Contents
1. [Overview](#overview)
2. [Core Features](#core-features)
3. [Strategies](#strategies)
   - [Role-based Strategy](#role-based-strategy)
   - [Debate Strategy](#debate-strategy)
   - [Attention Strategy](#attention-strategy)
   - [Combined Strategies](#combined-strategies)
   - [ReConcile Strategy](#reconcile-strategy)
4. [Token Usage Tracking](#token-usage-tracking)
5. [Performance Optimization](#performance-optimization)
6. [Best Practices](#best-practices)
7. [Examples](#examples)

## Overview

GaussMOA is a sophisticated Multi-Agent Orchestration framework implemented in Rust. It provides advanced strategies for coordinating multiple AI agents to work together effectively, with features like role-based specialization, multi-round debates, and comprehensive token usage tracking.

## Core Features

### 1. Role-based Agent Specialization
- Dynamic role assignment
- Specialization-aware routing
- Performance tracking per role
- Confidence thresholds
- Role-specific metrics

### 2. Multi-round Debate System
- Moderated agent discussions
- Quality and relevance scoring
- Automatic consensus detection
- Early termination optimization
- Debate history tracking

### 3. Token Usage Tracking
- Per-agent metrics
- Per-request metrics
- Global statistics
- Category-based tracking
- Cost analysis

### 4. Advanced Metrics
- Response quality evaluation
- Performance monitoring
- Resource utilization tracking
- Success rate analysis

## Strategies

### Role-based Strategy

The role-based strategy allows agents to specialize in specific domains:

```rust
let strategy = RoleStrategy::new(100); // max_history_size = 100

// Assign roles to agents
strategy.assign_role(
    "math_expert",
    AgentRole {
        name: "Mathematics Expert",
        description: "Specializes in mathematical problems",
        specializations: vec!["mathematics", "calculation"],
        confidence_threshold: 0.7,
    },
).await?;

// Select agents based on roles
let selected = strategy.select_agents(&request, &agents, 2).await?;
```

Key features:
- Role-based agent selection
- Specialization matching
- Performance history tracking
- Confidence thresholding

### Debate Strategy

The debate strategy enables multi-round discussions between agents:

```rust
let strategy = DebateStrategy::new(
    3,    // max_rounds
    0.5,  // min_confidence
    2,    // responses_per_round
);

// Start debate round
let responses = strategy.start_round(&debate_id, &request, &agents).await?;

// Check debate history
let history = strategy.get_debate_history(&debate_id).await?;
```

Key features:
- Multi-round discussions
- Response moderation
- Quality scoring
- Automatic consensus detection
- Debate history tracking

### Attention Strategy

The attention strategy uses multi-head attention for response aggregation:

```rust
let strategy = AttentionStrategy::new(
    4,     // num_heads
    128,   // embedding_dim
    0.01,  // learning_rate
    0.1,   // temperature
    100,   // max_history
);
```

Key features:
- Multi-head attention
- Dynamic weight updates
- Temperature scaling
- Performance-based learning

### Combined Strategies

Strategies can be combined for more sophisticated orchestration:

```rust
// 1. Select agents using role strategy
let selected = role_strategy.select_agents(&request, &agents, 3).await?;

// 2. Run debate with selected agents
let debate_responses = debate_strategy.start_round(&debate_id, &request, &selected).await?;

// 3. Use attention for final aggregation
let final_result = attention_strategy.process(&agents, &debate_responses).await?;
```

### ReConcile Strategy

The ReConcile strategy provides sophisticated consensus building and knowledge reconciliation:

```rust
let strategy = ReConcileStrategy::new(
    3,    // max_rounds
    0.5,  // min_confidence
    0.8,  // consensus_threshold
);

// Start reconciliation round
let responses = strategy.start_round(&reconciliation_id, &request, &agents).await?;

// Check reconciliation progress
let history = strategy.get_history(&reconciliation_id).await?;
```

Key features:
- Multi-round consensus building
- Conflict detection and resolution
- Knowledge base integration
- Perspective reconciliation
- Progress tracking

#### Conflict Types
The strategy handles different types of conflicts:
- **Factual**: Disagreements about facts
- **Logical**: Contradictions in reasoning
- **Perspective**: Different viewpoints
- **Incomplete**: Missing information

#### Resolution Process
1. **Conflict Detection**
   - Identify conflicts between responses
   - Classify conflict types
   - Track involved agents

2. **Knowledge Integration**
   - Maintain domain knowledge base
   - Track consensus patterns
   - Store resolution rules

3. **Resolution Rules**
   - Priority-based resolution
   - Evidence tracking
   - Success rate monitoring

4. **Consensus Building**
   - Agreement level tracking
   - Resolution rate monitoring
   - Early termination on consensus

#### Example Usage

```rust
// Initialize strategy
let strategy = ReConcileStrategy::new(3, 0.5, 0.8);

// Start reconciliation process
let mut round = 0;
while round < max_rounds {
    let responses = strategy.start_round(&reconciliation_id, &request, &agents).await?;
    
    // Check progress
    if let Some(history) = strategy.get_history(&reconciliation_id).await {
        if let Some(last_round) = history.last() {
            // Log metrics
            println!("Agreement level: {}", last_round.metrics.agreement_level);
            println!("Conflicts: {}", last_round.metrics.conflict_count);
            println!("Resolution rate: {}", last_round.metrics.resolution_rate);

            if last_round.consensus_reached {
                break;
            }
        }
    }
    round += 1;
}
```

#### Best Practices

1. **Conflict Resolution**
   - Define clear resolution rules
   - Set appropriate confidence thresholds
   - Monitor resolution success rates
   - Update rules based on performance

2. **Knowledge Management**
   - Maintain up-to-date knowledge base
   - Validate knowledge entries
   - Track knowledge sources
   - Regular cleanup of outdated entries

3. **Consensus Building**
   - Start with conservative thresholds
   - Adjust based on domain complexity
   - Monitor agreement trends
   - Use early termination when appropriate

4. **Performance Optimization**
   - Cache frequent knowledge lookups
   - Batch knowledge updates
   - Prune inactive entries
   - Monitor resource usage

## Token Usage Tracking

The framework provides comprehensive token usage tracking:

```rust
let tracker = TokenMetricsTracker::new();

// Track token usage
tracker.track_agent_response(
    agent_id,
    &request,
    &response,
    input_tokens,
    output_tokens,
    category,
).await?;

// Generate report
let report = tracker.generate_report().await;
```

Metrics tracked:
- Input/output tokens per agent
- Token usage by category
- Request-specific metrics
- Global statistics
- Cost analysis

## Performance Optimization

1. **Role-based Optimization**
   - Use appropriate confidence thresholds
   - Adjust specialization matching
   - Monitor role performance

2. **Debate Optimization**
   - Set appropriate round limits
   - Tune moderation parameters
   - Enable early termination

3. **Token Usage Optimization**
   - Monitor token consumption
   - Optimize request batching
   - Use token rate limiting

## Best Practices

1. **Role Assignment**
   - Define clear specializations
   - Set appropriate confidence thresholds
   - Monitor role performance
   - Update roles based on metrics

2. **Debate Configuration**
   - Start with conservative round limits
   - Adjust moderation thresholds
   - Enable early termination
   - Monitor debate quality

3. **Token Management**
   - Track token usage patterns
   - Set appropriate budgets
   - Monitor costs
   - Optimize request batching

4. **Error Handling**
   - Implement proper fallbacks
   - Handle timeouts gracefully
   - Monitor error patterns
   - Log relevant metrics

## Examples

See the `examples/advanced_moa.rs` file for a comprehensive example demonstrating:
- Role-based agent specialization
- Multi-round debates
- Token usage tracking
- Combined strategy usage
- Performance monitoring

Example usage:
```rust
// Initialize strategies
let role_strategy = RoleStrategy::new(100);
let debate_strategy = DebateStrategy::new(3, 0.5, 2);
let attention_strategy = AttentionStrategy::new(4, 128, 0.01, 0.1, 100);

// Process request with combined strategies
let selected = role_strategy.select_agents(&request, &agents, 3).await?;
let debate_responses = debate_strategy.start_round(&debate_id, &request, &selected).await?;
let final_result = attention_strategy.process(&agents, &debate_responses).await?;

// Track token usage
token_tracker.track_agent_response(
    agent.id(),
    &request,
    &response,
    input_tokens,
    output_tokens,
    "combined_strategy",
).await?;
```

For more examples and detailed API documentation, see the [API Reference](./api_reference.md). 