use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentVersion {
    pub component: String,
    pub version: Version,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemVersion {
    pub core: Version,
    pub components: Vec<ComponentVersion>,
    pub api_version: Version,
    pub config_version: Version,
}

impl SystemVersion {
    pub fn current() -> Self {
        Self {
            core: Version::new(0, 1, 0),
            components: vec![
                ComponentVersion {
                    component: "agent_manager".to_string(),
                    version: Version::new(0, 1, 0),
                    features: vec!["discovery".to_string(), "negotiation".to_string()],
                },
                ComponentVersion {
                    component: "strategy_manager".to_string(),
                    version: Version::new(0, 1, 0),
                    features: vec!["standard".to_string(), "sparse".to_string(), "self_moa".to_string()],
                },
                ComponentVersion {
                    component: "resource_manager".to_string(),
                    version: Version::new(0, 1, 0),
                    features: vec!["caching".to_string(), "backpressure".to_string()],
                },
            ],
            api_version: Version::new(0, 1, 0),
            config_version: Version::new(0, 1, 0),
        }
    }

    pub fn is_compatible_with(&self, other: &SystemVersion) -> bool {
        self.core.major == other.core.major &&
        self.api_version.major == other.api_version.major &&
        self.config_version.major == other.config_version.major
    }

    pub fn requires_migration(&self, other: &SystemVersion) -> bool {
        self.config_version > other.config_version
    }
}

impl fmt::Display for SystemVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MOA v{} (API v{}, Config v{})", 
            self.core, self.api_version, self.config_version)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub system: SystemVersion,
    pub build_timestamp: String,
}

impl VersionInfo {
    pub fn new() -> Self {
        Self {
            system: SystemVersion::current(),
            build_timestamp: Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        let current = SystemVersion::current();
        let mut incompatible = current.clone();
        incompatible.core = Version::new(1, 0, 0);

        assert!(current.is_compatible_with(&current));
        assert!(!current.is_compatible_with(&incompatible));
    }

    #[test]
    fn test_version_migration() {
        let current = SystemVersion::current();
        let mut older = current.clone();
        older.config_version = Version::new(0, 0, 1);

        assert!(current.requires_migration(&older));
        assert!(!older.requires_migration(&current));
    }
} 