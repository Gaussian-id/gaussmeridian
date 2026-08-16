# Plugin Development Guide

GaussMeridian supports powerful plugin extension via transform and middleware plugins.

## Plugin Architecture
- **TransformPlugin**: Mutate requests/responses (e.g. web search, translation)
- **MiddlewarePlugin**: Pre/post-process, enforce policies (e.g. rate limiting, auth)
- Plugins are loaded and managed by the plugin registry.

## Writing a Plugin
1. Implement the appropriate trait (`TransformPlugin` or `MiddlewarePlugin`)
2. Register your plugin in config or at runtime
3. Validate and test your plugin

### Example: Transform Plugin
```rust
use async_trait::async_trait;
use gaussmeridian::plugins::traits::{TransformPlugin, TransformContext, TransformError};

pub struct MyTransform;

#[async_trait]
impl TransformPlugin for MyTransform {
    fn name(&self) -> &str { "my_transform" }
    fn version(&self) -> &str { "1.0.0" }
    async fn transform_request(&self, req, ctx) -> Result<(), TransformError> {
        // mutate req
        Ok(())
    }
    async fn transform_response(&self, resp, ctx) -> Result<(), TransformError> {
        // mutate resp
        Ok(())
    }
    fn get_config(&self) -> serde_json::Value { serde_json::json!({}) }
    fn validate_config(&self, _c) -> Result<(), TransformError> { Ok(()) }
}
```

## Best Practices
- Keep plugins stateless or use thread-safe state (Arc, Mutex)
- Validate all input/output
- Handle errors gracefully
- Avoid blocking operations; use async
- Follow SOLID and Rust design patterns

## Security
- Never trust user input; validate and sanitize
- Avoid leaking secrets/logging sensitive data
- Use least privilege for external calls

## Testing
- Use the testing framework (`src/testing/`)
- Provide unit and integration tests

## See Also
- `src/plugins/traits.rs` for trait definitions
- Example plugins in `plugins/` directory 