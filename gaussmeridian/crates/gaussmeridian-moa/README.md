# GaussMoA (Gaussian Mixture of Agents)

A robust, scalable framework for orchestrating and managing multiple AI agents with advanced strategies, metrics, and response processing capabilities.

## Features

### Core Features
- **Multi-Agent Orchestration**: Coordinate multiple AI agents with different roles and capabilities
- **Strategy-based Processing**: Multiple implemented strategies for response processing:
  - Standard Strategy: Basic confidence-based selection
  - Attention Strategy: Multi-head attention mechanism for response weighting
  - Debate Strategy: Consensus-driven response selection
  - Roles Strategy: Role-based agent specialization
  - Sparse Strategy: Top-K filtering with confidence thresholds
  - Self-MoA Strategy: Multi-round self-improvement
  - Collaborative Strategy: Agent collaboration framework
  - Adaptive Strategy: Performance-based adaptation
- **Metrics Collection**: Comprehensive metrics tracking with Prometheus integration
- **Response Processing**: Advanced response handling with confidence scoring
- **Agent Management**: Flexible agent configuration and role assignment

### Technical Features
- **Async/Await**: Built on Tokio for high-performance async operations
- **Error Handling**: Comprehensive error types and handling mechanisms
- **Metrics**: Built-in metrics collection and Prometheus integration
- **Testing**: Unit tests for core components and strategies
- **Type Safety**: Strong type system throughout the codebase

## Installation

### Prerequisites

- Rust 1.70 or higher
- Cargo

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/gaussmoa.git
cd gaussmoa

# Build the project
cargo build --release

# Run tests
cargo test
```

## Configuration

GaussMoA uses a flexible configuration system that supports:

1. Agent configuration
2. Strategy selection and configuration
3. Metrics collection settings
4. Response processing parameters

### Example Configuration

```rust
use gaussmoa::{
    config::AgentConfig,
    strategies::StandardStrategy,
    metrics::MetricsRegistry,
};

// Configure an agent
let agent_config = AgentConfig {
    name: "primary_agent".to_string(),
    role: AgentRole::Primary,
    // ... other configuration
};

// Select a strategy
let strategy = StandardStrategy::default();

// Configure metrics
let metrics = MetricsRegistry::new()?;
```

## Usage

### Basic Usage

```rust
use gaussmoa::{MoaRequest, AgentResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a request
    let request = MoaRequest::new(
        "Your query here".to_string(),
        None,
    );
    
    // Process request through strategy
    let responses = vec![
        // ... collect responses from agents
    ];
    
    let result = strategy.process_responses(responses, &request).await?;
    
    println!("Response: {}", result.content);
    Ok(())
}
```

## Implemented Strategies

### Standard Strategy
Basic confidence-based selection of responses.

### Attention Strategy
Multi-head attention mechanism for weighted response selection.

### Debate Strategy
Consensus-driven response selection with configurable rounds.

### Roles Strategy
Role-based agent specialization with adaptive skill profiles.

### Sparse Strategy
Top-K filtering with configurable confidence thresholds.

### Self-MoA Strategy
Multi-round self-improvement with sampling.

### Collaborative Strategy
Agent collaboration framework with shared context.

### Adaptive Strategy
Performance-based adaptation with historical tracking.

## Metrics and Monitoring

GaussMoA provides metrics through Prometheus integration:

- Response processing metrics
- Strategy performance metrics
- Agent performance metrics
- System health metrics

## Contributing

Contributions are welcome! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

## License

GNU Affero General Public License v3.0 only (`AGPL-3.0-only`), inherited from the
GaussMeridian workspace — see the repository-root [`LICENSE`](../../../LICENSE). This
crate carries no separate license of its own.

Because `gaussmeridian-moa` runs inside a network-facing gateway, AGPL-3.0 Section 13
reaches it: if you modify this crate and expose the orchestration service to users
over a network, you must offer those users the Corresponding Source of your modified
build. See the repository-root [`NOTICE`](../../../NOTICE) for how GaussMeridian
publishes that offer.

## Acknowledgments

- The Rust community
- Contributors and maintainers