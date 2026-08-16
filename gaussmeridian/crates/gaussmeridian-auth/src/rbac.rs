//! Role-Based Access Control (RBAC) implementation
//!
//! This module provides comprehensive RBAC with resource-level permissions,
//! role hierarchies, and policy-based access control.

use crate::error::AuthError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, warn};

/// Standard roles in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StandardRole {
    Administrator,
    Developer,
    Viewer,
}

impl StandardRole {
    /// Get default permissions for a standard role
    pub fn default_permissions(&self) -> Vec<Permission> {
        match self {
            StandardRole::Administrator => vec![Permission::All],
            StandardRole::Developer => vec![
                Permission::Read,
                Permission::Write,
                Permission::Execute,
                Permission::Create,
                Permission::Update,
                Permission::Delete,
                Permission::ModelsRead,
                Permission::ModelsWrite,
                Permission::ProvidersRead,
                Permission::ProvidersWrite,
                Permission::AnalyticsRead,
            ],
            StandardRole::Viewer => vec![
                Permission::Read,
                Permission::ModelsRead,
                Permission::ProvidersRead,
                Permission::AnalyticsRead,
            ],
        }
    }
}

/// Permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// Full access to all resources
    All,
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute access
    Execute,
    /// Create access
    Create,
    /// Update access
    Update,
    /// Delete access
    Delete,
    /// Model management - read
    ModelsRead,
    /// Model management - write
    ModelsWrite,
    /// Provider management - read
    ProvidersRead,
    /// Provider management - write
    ProvidersWrite,
    /// Analytics - read
    AnalyticsRead,
    /// Analytics - write
    AnalyticsWrite,
    /// User management - read
    UsersRead,
    /// User management - write
    UsersWrite,
    /// Tenant management - read
    TenantsRead,
    /// Tenant management - write
    TenantsWrite,
    /// Agent management - read
    AgentsRead,
    /// Agent management - write
    AgentsWrite,
    /// Configuration management - read
    ConfigRead,
    /// Configuration management - write
    ConfigWrite,
    /// Custom permission (with name)
    Custom(String),
}

impl Permission {
    /// Check if a permission includes another permission
    pub fn includes(&self, other: &Permission) -> bool {
        match (self, other) {
            (Permission::All, _) => true,
            (a, b) => a == b,
        }
    }
}

/// Resource identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: String,
    pub resource_id: Option<String>,
}

impl Resource {
    pub fn new(resource_type: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            resource_id: None,
        }
    }

    pub fn with_id(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            resource_id: Some(resource_id.into()),
        }
    }
}

/// Policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub resource: Resource,
    pub permissions: Vec<Permission>,
    pub conditions: Vec<PolicyCondition>,
}

/// Policy condition for conditional access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyCondition {
    TenantMatches(String),
    UserMatches(String),
    TimeRange { start: String, end: String }, // ISO 8601 format
    IPWhitelist(Vec<String>),
    Custom(String),
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub policies: Vec<PolicyRule>,
    pub inherits_from: Vec<String>, // Role IDs to inherit from
}

impl Role {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            permissions: Vec::new(),
            policies: Vec::new(),
            inherits_from: Vec::new(),
        }
    }

    pub fn with_permissions(mut self, permissions: Vec<Permission>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_policies(mut self, policies: Vec<PolicyRule>) -> Self {
        self.policies = policies;
        self
    }

    pub fn inherit_from(mut self, role_id: impl Into<String>) -> Self {
        self.inherits_from.push(role_id.into());
        self
    }
}

/// Access context for permission checks
#[derive(Debug, Clone)]
pub struct AccessContext {
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub roles: Vec<String>,
    pub ip_address: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl AccessContext {
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            tenant_id: None,
            roles: Vec::new(),
            ip_address: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_role(mut self, role_id: impl Into<String>) -> Self {
        self.roles.push(role_id.into());
        self
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }
}

/// RBAC manager
pub struct RBACManager {
    roles: HashMap<String, Role>,
    role_hierarchy: HashMap<String, HashSet<String>>,
}

impl RBACManager {
    /// Create a new RBAC manager
    pub fn new() -> Self {
        let mut manager = Self {
            roles: HashMap::new(),
            role_hierarchy: HashMap::new(),
        };

        // Initialize standard roles
        manager.init_standard_roles();
        manager
    }

    fn init_standard_roles(&mut self) {
        let admin_role = Role::new("administrator", "Administrator")
            .with_permissions(StandardRole::Administrator.default_permissions());
        self.register_role(admin_role);

        let dev_role = Role::new("developer", "Developer")
            .with_permissions(StandardRole::Developer.default_permissions());
        self.register_role(dev_role);

        let viewer_role = Role::new("viewer", "Viewer")
            .with_permissions(StandardRole::Viewer.default_permissions());
        self.register_role(viewer_role);
    }

    /// Register a role
    pub fn register_role(&mut self, role: Role) {
        let role_id = role.id.clone();
        let inherits_from = role.inherits_from.clone();

        // Build role hierarchy
        for parent_id in inherits_from {
            self.role_hierarchy
                .entry(role_id.clone())
                .or_default()
                .insert(parent_id);
        }

        self.roles.insert(role_id, role);
    }

    /// Get a role by ID
    pub fn get_role(&self, role_id: &str) -> Option<&Role> {
        self.roles.get(role_id)
    }

    /// Get all permissions for a role (including inherited)
    pub fn get_role_permissions(&self, role_id: &str) -> HashSet<Permission> {
        let mut permissions = HashSet::new();
        self.collect_permissions(role_id, &mut permissions, &mut HashSet::new());
        permissions
    }

    fn collect_permissions(
        &self,
        role_id: &str,
        permissions: &mut HashSet<Permission>,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(role_id) {
            return; // Prevent circular dependencies
        }
        visited.insert(role_id.to_string());

        if let Some(role) = self.roles.get(role_id) {
            // Add direct permissions
            for perm in &role.permissions {
                permissions.insert(perm.clone());
            }

            // Add inherited permissions
            if let Some(parents) = self.role_hierarchy.get(role_id) {
                for parent_id in parents {
                    self.collect_permissions(parent_id, permissions, visited);
                }
            }
        }
    }

    /// Check if a context has permission for a resource
    pub fn check_permission(
        &self,
        context: &AccessContext,
        resource: &Resource,
        permission: &Permission,
    ) -> Result<bool, AuthError> {
        // Collect all permissions from all roles
        let mut all_permissions = HashSet::new();
        for role_id in &context.roles {
            let role_perms = self.get_role_permissions(role_id);
            all_permissions.extend(role_perms);
        }

        // Check if permission is granted through role permissions
        let has_permission = all_permissions.iter().any(|perm| perm.includes(permission));
        
        if !has_permission {
            return Ok(false);
        }

        // Check policies for additional conditions (if any policies apply)
        // If no policies are defined for this permission, the permission is granted
        self.check_policies(context, resource, permission)
    }

    fn check_policies(
        &self,
        context: &AccessContext,
        resource: &Resource,
        permission: &Permission,
    ) -> Result<bool, AuthError> {
        // Collect all applicable policies from all roles
        let mut applicable_policies = Vec::new();
        let mut matching_policies = Vec::new();
        
        for role_id in &context.roles {
            if let Some(role) = self.roles.get(role_id) {
                for policy in &role.policies {
                    // Check if policy applies to this resource
                    if policy.resource.resource_type == resource.resource_type {
                        let resource_matches = match (&policy.resource.resource_id, &resource.resource_id) {
                            (Some(policy_id), Some(resource_id)) => {
                                policy_id == resource_id || policy_id == "*"
                            }
                            (Some(policy_id), None) => policy_id == "*",
                            (None, _) => true, // No specific resource_id means it applies to all
                        };

                        if resource_matches {
                            // Check if policy includes the permission
                            if policy.permissions.iter().any(|p| p.includes(permission)) {
                                applicable_policies.push(policy);
                                // Check conditions
                                if self.check_conditions(context, &policy.conditions)? {
                                    matching_policies.push(policy);
                                }
                            }
                        }
                    }
                }
            }
        }

        // If no policies are applicable, permission is granted (role permission is sufficient)
        // If policies are applicable, at least one must match for permission to be granted
        if applicable_policies.is_empty() {
            Ok(true)
        } else {
            Ok(!matching_policies.is_empty())
        }
    }

    fn check_conditions(
        &self,
        context: &AccessContext,
        conditions: &[PolicyCondition],
    ) -> Result<bool, AuthError> {
        for condition in conditions {
            match condition {
                PolicyCondition::TenantMatches(tenant_id) => {
                    if context.tenant_id.as_ref() != Some(tenant_id) {
                        return Ok(false);
                    }
                }
                PolicyCondition::UserMatches(user_id) => {
                    if context.user_id != *user_id {
                        return Ok(false);
                    }
                }
                PolicyCondition::IPWhitelist(whitelist) => {
                    if let Some(ref ip) = context.ip_address {
                        if !whitelist.contains(ip) {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                }
                PolicyCondition::TimeRange { start, end } => {
                    // Time-based access control (simplified - in production, parse and check)
                    debug!("Time-based condition check: {} - {}", start, end);
                    // For now, allow all time ranges
                }
                PolicyCondition::Custom(_) => {
                    // Custom condition logic would go here
                    warn!("Custom policy condition not implemented");
                }
            }
        }
        Ok(true)
    }

    /// Check if context has any of the specified permissions
    pub fn has_any_permission(
        &self,
        context: &AccessContext,
        resource: &Resource,
        permissions: &[Permission],
    ) -> Result<bool, AuthError> {
        for permission in permissions {
            if self.check_permission(context, resource, permission)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if context has all of the specified permissions
    pub fn has_all_permissions(
        &self,
        context: &AccessContext,
        resource: &Resource,
        permissions: &[Permission],
    ) -> Result<bool, AuthError> {
        for permission in permissions {
            if !self.check_permission(context, resource, permission)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl Default for RBACManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_includes() {
        assert!(Permission::All.includes(&Permission::Read));
        assert!(Permission::Read.includes(&Permission::Read));
        assert!(!Permission::Read.includes(&Permission::Write));
    }

    #[test]
    fn test_role_hierarchy() {
        let mut manager = RBACManager::new();

        // Create a custom role that inherits from developer
        let custom_role = Role::new("custom", "Custom Role")
            .with_permissions(vec![Permission::Custom("special".to_string())])
            .inherit_from("developer");

        manager.register_role(custom_role);

        // Custom role should have both its own permissions and developer permissions
        let permissions = manager.get_role_permissions("custom");
        assert!(permissions.contains(&Permission::Custom("special".to_string())));
        assert!(permissions.contains(&Permission::Read));
        assert!(permissions.contains(&Permission::Write));
    }

    #[test]
    fn test_permission_check() {
        let manager = RBACManager::new();
        let context = AccessContext::new("user1").with_role("developer");

        let resource = Resource::new("model");
        assert!(manager
            .check_permission(&context, &resource, &Permission::Read)
            .unwrap());
        assert!(manager
            .check_permission(&context, &resource, &Permission::Write)
            .unwrap());

        let viewer_context = AccessContext::new("user2").with_role("viewer");
        assert!(manager
            .check_permission(&viewer_context, &resource, &Permission::Read)
            .unwrap());
        assert!(!manager
            .check_permission(&viewer_context, &resource, &Permission::Write)
            .unwrap());
    }
}
