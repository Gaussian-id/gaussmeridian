//! Plugin manager for handling plugin lifecycle

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::{error::PluginError, traits::Plugin, types::PluginConfig};

/// Plugin manager for handling plugin lifecycle
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
    configs: Arc<RwLock<HashMap<String, PluginConfig>>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Result<Self, PluginError> {
        Ok(Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Register a plugin
    pub async fn register_plugin(
        &self,
        name: String,
        plugin: Box<dyn Plugin>,
    ) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;

        if plugins.contains_key(&name) {
            return Err(PluginError::PluginAlreadyLoaded(name));
        }

        info!("Registering plugin: {}", name);
        plugins.insert(name.clone(), plugin);

        Ok(())
    }

    /// Unregister a plugin
    pub async fn unregister_plugin(&self, name: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;

        if let Some(mut plugin) = plugins.remove(name) {
            info!("Unregistering plugin: {}", name);
            if let Err(e) = plugin.shutdown().await {
                error!("Failed to shutdown plugin {}: {}", name, e);
            }
        }

        Ok(())
    }

    /// Get a plugin by name
    pub async fn get_plugin(&self, name: &str) -> bool {
        let plugins = self.plugins.read().await;
        plugins.contains_key(name)
    }

    /// List all registered plugins
    pub async fn list_plugins(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }

    /// Enable a plugin
    pub async fn enable_plugin(&self, name: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(name) {
            plugin.set_enabled(true);
            info!("Enabled plugin: {}", name);
        } else {
            return Err(PluginError::PluginNotFound(name.to_string()));
        }

        Ok(())
    }

    /// Disable a plugin
    pub async fn disable_plugin(&self, name: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(name) {
            plugin.set_enabled(false);
            info!("Disabled plugin: {}", name);
        } else {
            return Err(PluginError::PluginNotFound(name.to_string()));
        }

        Ok(())
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&self) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;

        for (name, mut plugin) in plugins.drain() {
            info!("Shutting down plugin: {}", name);
            if let Err(e) = plugin.shutdown().await {
                error!("Failed to shutdown plugin {}: {}", name, e);
            }
        }

        Ok(())
    }
}
