//! Tests for the plugin system

use async_trait::async_trait;

use crate::{error::PluginError, manager::PluginManager, traits::Plugin, types::PluginConfig};

struct MockPlugin {
    name: String,
    enabled: bool,
}

#[async_trait]
impl Plugin for MockPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "A mock plugin for testing"
    }

    async fn initialize(&mut self, _config: PluginConfig) -> Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[tokio::test]
async fn test_plugin_manager() {
    let manager = PluginManager::new().unwrap();

    let plugin = Box::new(MockPlugin {
        name: "test".to_string(),
        enabled: true,
    });

    manager
        .register_plugin("test".to_string(), plugin)
        .await
        .unwrap();

    let plugins = manager.list_plugins().await;
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0], "test");

    manager.unregister_plugin("test").await.unwrap();

    let plugins = manager.list_plugins().await;
    assert_eq!(plugins.len(), 0);
}
