//! Plugin system for GaussMeridian
//!
//! This crate provides a plugin system that allows extending GaussMeridian
//! functionality through dynamic loading and hot-reloading of plugins.

pub mod error;
pub mod manager;
pub mod registry;
pub mod traits;
pub mod types;

pub use error::PluginError;
pub use manager::PluginManager;
pub use registry::PluginRegistry;
pub use traits::{MiddlewarePlugin, Plugin, RequestTransformPlugin, ResponseTransformPlugin};
pub use types::PluginConfig;

#[cfg(test)]
mod tests;
