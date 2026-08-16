# Provider Development Guide

GaussMeridian supports custom LLM providers via the `LLMProvider` trait.

## Provider Architecture
- Each provider implements `LLMProvider` (async, thread-safe)
- Providers are registered at startup or dynamically
- Provider registry manages routing and health

## Writing a Provider
1. Implement the `LLMProvider` trait (see `src/providers/traits.rs`)
2. Register your provider in config or at runtime
3. Implement all required methods (chat, completion, embedding, streaming, health, etc.)

### Example: Minimal Provider
```rust
use async_trait::async_trait;
use gaussmeridian::providers::traits::{LLMProvider, ProviderError};

pub struct MyProvider;

#[async_trait]
impl LLMProvider for MyProvider {
    type Error = ProviderError;
    async fn chat_completion(&self, req) -> Result<_, _> { unimplemented!() }
    async fn chat_completion_stream(&self, req) -> Result<_, _> { unimplemented!() }
    async fn completion(&self, req) -> Result<_, _> { unimplemented!() }
    async fn completion_stream(&self, req) -> Result<_, _> { unimplemented!() }
    async fn embedding(&self, req) -> Result<_, _> { unimplemented!() }
    async fn list_models(&self) -> Result<_, _> { unimplemented!() }
    fn metadata(&self) -> _ { unimplemented!() }
    async fn health_check(&self) -> Result<(), _> { Ok(()) }
    fn capabilities(&self) -> _ { unimplemented!() }
    async fn get_cost_info(&self, _model) -> Result<_, _> { unimplemented!() }
    async fn supports_model(&self, _model) -> bool { false }
    fn get_config(&self) -> _ { unimplemented!() }
}
```

## Best Practices
- Use async for all I/O
- Use Arc for shared state, avoid global mutability
- Validate all input/output
- Handle errors and timeouts robustly
- Follow SOLID and Rust design patterns

## Security
- Never log secrets or API keys
- Use secure HTTP clients (TLS, timeouts)
- Validate all external data

## Testing
- Use the testing framework (`src/testing/`)
- Provide unit and integration tests

## See Also
- `src/providers/traits.rs` for trait definitions
- Example providers in `src/providers/` directory 