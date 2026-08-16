use async_trait::async_trait;
use gaussmeridian_server::routing::identity::{
    CanonicalProjectResolver, IdentityError, IdentityStoreError, ProjectIdentityStore,
    ProjectScope, ResolveProjectInput,
};

#[derive(Default)]
struct StubIdentityStore {
    api_key_scope: Option<ProjectScope>,
    accessible_projects: Vec<ProjectScope>,
    membership: Option<ProjectScope>,
    store_error: bool,
}

#[async_trait]
impl ProjectIdentityStore for StubIdentityStore {
    async fn api_key_scope(
        &self,
        _api_key_id: &str,
    ) -> Result<Option<ProjectScope>, IdentityStoreError> {
        if self.store_error {
            return Err(IdentityStoreError {
                message: "database unavailable".to_string(),
            });
        }
        Ok(self.api_key_scope.clone())
    }

    async fn accessible_projects(
        &self,
        _user_id: &str,
    ) -> Result<Vec<ProjectScope>, IdentityStoreError> {
        if self.store_error {
            return Err(IdentityStoreError {
                message: "database unavailable".to_string(),
            });
        }
        Ok(self.accessible_projects.clone())
    }

    async fn verify_membership(
        &self,
        _user_id: &str,
        _project_id: &str,
    ) -> Result<Option<ProjectScope>, IdentityStoreError> {
        if self.store_error {
            return Err(IdentityStoreError {
                message: "database unavailable".to_string(),
            });
        }
        Ok(self.membership.clone())
    }
}

#[tokio::test]
async fn active_scoped_api_key_resolves_its_exact_project() {
    let resolver = CanonicalProjectResolver::new(StubIdentityStore {
        api_key_scope: Some(ProjectScope {
            project_id: "project-a".to_string(),
            org_id: "org-a".to_string(),
            active: true,
        }),
        ..StubIdentityStore::default()
    });

    let identity = resolver
        .resolve(ResolveProjectInput::ApiKey {
            api_key_id: "key-a",
        })
        .await
        .expect("active scoped API key resolves");

    assert_eq!(identity.project_id, "project-a");
    assert_eq!(identity.org_id, "org-a");
}

#[tokio::test]
async fn explicit_jwt_project_resolves_only_through_verified_membership() {
    let resolver = CanonicalProjectResolver::new(StubIdentityStore {
        membership: Some(ProjectScope {
            project_id: "project-b".to_string(),
            org_id: "org-b".to_string(),
            active: true,
        }),
        ..StubIdentityStore::default()
    });

    let identity = resolver
        .resolve(ResolveProjectInput::Jwt {
            user_id: "user-b",
            requested_project_id: Some("project-b"),
        })
        .await
        .expect("verified explicit JWT project resolves");

    assert_eq!(identity.project_id, "project-b");
    assert_eq!(identity.principal_id, "user-b");
}

#[tokio::test]
async fn jwt_without_explicit_context_resolves_only_when_one_project_is_accessible() {
    let resolver = CanonicalProjectResolver::new(StubIdentityStore {
        accessible_projects: vec![ProjectScope {
            project_id: "project-only".to_string(),
            org_id: "org-only".to_string(),
            active: true,
        }],
        ..StubIdentityStore::default()
    });

    let identity = resolver
        .resolve(ResolveProjectInput::Jwt {
            user_id: "user-only",
            requested_project_id: None,
        })
        .await
        .expect("exactly one accessible project is canonical");

    assert_eq!(identity.project_id, "project-only");
}

#[tokio::test]
async fn ambiguous_jwt_project_context_is_rejected() {
    let resolver = CanonicalProjectResolver::new(StubIdentityStore {
        accessible_projects: vec![
            ProjectScope {
                project_id: "project-a".to_string(),
                org_id: "org-a".to_string(),
                active: true,
            },
            ProjectScope {
                project_id: "project-b".to_string(),
                org_id: "org-b".to_string(),
                active: true,
            },
        ],
        ..StubIdentityStore::default()
    });

    let error = resolver
        .resolve(ResolveProjectInput::Jwt {
            user_id: "user-multi",
            requested_project_id: None,
        })
        .await
        .expect_err("arbitrary first-project fallback is forbidden");

    assert_eq!(error, IdentityError::ProjectContextRequired);
}

#[tokio::test]
async fn unscoped_api_key_is_rejected() {
    let resolver = CanonicalProjectResolver::new(StubIdentityStore::default());

    let error = resolver
        .resolve(ResolveProjectInput::ApiKey {
            api_key_id: "unscoped-key",
        })
        .await
        .expect_err("generation keys must be project scoped");

    assert_eq!(error, IdentityError::UnscopedApiKey);
}

#[tokio::test]
async fn inactive_project_is_rejected() {
    let resolver = CanonicalProjectResolver::new(StubIdentityStore {
        api_key_scope: Some(ProjectScope {
            project_id: "inactive-project".to_string(),
            org_id: "org-a".to_string(),
            active: false,
        }),
        ..StubIdentityStore::default()
    });

    let error = resolver
        .resolve(ResolveProjectInput::ApiKey {
            api_key_id: "inactive-key",
        })
        .await
        .expect_err("inactive project cannot route");

    assert_eq!(error, IdentityError::InactiveProject);
}

#[tokio::test]
async fn identity_store_failure_is_authoritative_and_fails_closed() {
    let resolver = CanonicalProjectResolver::new(StubIdentityStore {
        store_error: true,
        ..StubIdentityStore::default()
    });

    let error = resolver
        .resolve(ResolveProjectInput::ApiKey {
            api_key_id: "key-a",
        })
        .await
        .expect_err("authoritative identity state cannot fail open");

    assert_eq!(error, IdentityError::AuthoritativeStateUnavailable);
}
