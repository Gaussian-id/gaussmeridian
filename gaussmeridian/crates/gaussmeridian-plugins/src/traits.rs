//! Plugin traits and interfaces

use async_trait::async_trait;
use gaussmeridian_models::{
    ChatCompletionRequest, ChatCompletionResponse, CompletionRequest, CompletionResponse,
};

use crate::{error::PluginError, types::PluginConfig};

/// Plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get the plugin name
    fn name(&self) -> &str;

    /// Get the plugin version
    fn version(&self) -> &str;

    /// Get the plugin description
    fn description(&self) -> &str;

    /// Initialize the plugin
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), PluginError>;

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<(), PluginError>;

    /// Check if the plugin is enabled
    fn is_enabled(&self) -> bool;

    /// Enable or disable the plugin
    fn set_enabled(&mut self, enabled: bool);
}

/// Request transformation plugin trait
#[async_trait]
pub trait RequestTransformPlugin: Plugin {
    /// Transform a chat completion request
    async fn transform_chat_request(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionRequest, PluginError>;

    /// Transform a completion request
    async fn transform_completion_request(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionRequest, PluginError>;
}

/// Response transformation plugin trait
#[async_trait]
pub trait ResponseTransformPlugin: Plugin {
    /// Transform a chat completion response
    async fn transform_chat_response(
        &self,
        response: ChatCompletionResponse,
    ) -> Result<ChatCompletionResponse, PluginError>;

    /// Transform a completion response
    async fn transform_completion_response(
        &self,
        response: CompletionResponse,
    ) -> Result<CompletionResponse, PluginError>;
}

/// Middleware plugin trait
#[async_trait]
pub trait MiddlewarePlugin: Plugin {
    /// Process a request before it's sent to the provider
    async fn process_request(&self, request: &ChatCompletionRequest) -> Result<(), PluginError>;

    /// Process a response after it's received from the provider
    async fn process_response(&self, response: &ChatCompletionResponse) -> Result<(), PluginError>;
}
