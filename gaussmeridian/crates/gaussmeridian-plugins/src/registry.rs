//! Plugin registry for managing plugin instances

use std::sync::Arc;

use crate::manager::PluginManager;

/// Plugin registry for managing plugin instances
pub struct PluginRegistry {
    manager: Arc<PluginManager>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new(manager: Arc<PluginManager>) -> Self {
        Self { manager }
    }

    /// Get the plugin manager
    pub fn manager(&self) -> &Arc<PluginManager> {
        &self.manager
    }
}
