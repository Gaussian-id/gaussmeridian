use async_trait::async_trait;
use gaussmeridian_auth::{AuthContext, CredentialKind};
use gaussmeridian_server::routing::{
    identity::{AuthKind, IdentityError, ResolveProjectInput, ResolvedProjectIdentity},
    management_identity::{requires_authoritative_project_identity, resolve_authoritative_project},
    snapshots::RoutingIdentityResolver,
};
use serde_json::json;
use std::{collections::HashMap, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum SeenInput {
    Jwt {
        user_id: String,
        requested_project_id: Option<String>,
    },
    ApiKey {
        api_key_id: String,
    },
}

struct MatrixResolver {
    seen: Mutex<Vec<SeenInput>>,
}

impl MatrixResolver {
    fn new() -> Self {
        Self {
            seen: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl RoutingIdentityResolver for MatrixResolver {
    async fn resolve_identity(
        &self,
        input: ResolveProjectInput<'_>,
    ) -> Result<ResolvedProjectIdentity, IdentityError> {
        let (seen, project_id, principal_id, auth_kind) = match input {
            ResolveProjectInput::Jwt {
                user_id,
                requested_project_id,
            } => {
                let Some(project_id) = requested_project_id else {
                    return Err(IdentityError::ProjectContextRequired);
                };
                (
                    SeenInput::Jwt {
                        user_id: user_id.to_string(),
                        requested_project_id: Some(project_id.to_string()),
                    },
                    project_id.to_string(),
                    user_id.to_string(),
                    AuthKind::Jwt,
                )
            }
            ResolveProjectInput::ApiKey { api_key_id } => (
                SeenInput::ApiKey {
                    api_key_id: api_key_id.to_string(),
                },
                "project-a".to_string(),
                api_key_id.to_string(),
                AuthKind::ApiKey,
            ),
        };
        self.seen.lock().expect("seen lock").push(seen);
        Ok(ResolvedProjectIdentity {
            project_id,
            org_id: "org-a".to_string(),
            principal_id,
            auth_kind,
        })
    }
}

fn jwt_auth() -> AuthContext {
    AuthContext {
        api_key: "jwt:session".to_string(),
        credential_kind: CredentialKind::Jwt,
        user_id: Some("user-a".to_string()),
        tenant_id: None,
        permissions: Vec::new(),
        metadata: HashMap::new(),
    }
}

fn api_key_auth() -> AuthContext {
    AuthContext {
        api_key: "gr_live_secret".to_string(),
        credential_kind: CredentialKind::ApiKey,
        user_id: Some("user-a".to_string()),
        tenant_id: None,
        permissions: Vec::new(),
        metadata: HashMap::from([("api_key_id".to_string(), json!("key-a"))]),
    }
}

#[tokio::test]
async fn one_explicit_jwt_project_is_authoritative_across_every_p0_surface() {
    let resolver = MatrixResolver::new();
    let surfaces = [
        "/v1/chat/completions",
        "/v1/project/settings",
        "/v1/byok/keys",
        "/v1/billing/budget",
        "/v1/route-decisions/request-a",
        "/v1/logs",
    ];

    for surface in surfaces {
        assert!(requires_authoritative_project_identity(surface));
        let identity =
            resolve_authoritative_project(Some(&resolver), &jwt_auth(), Some("project-a"))
                .await
                .expect("explicit project A resolves");
        assert_eq!(identity.project_id, "project-a", "surface {surface}");
    }
}

#[tokio::test]
async fn ambiguous_jwt_and_unavailable_authority_fail_closed() {
    let resolver = MatrixResolver::new();
    assert_eq!(
        resolve_authoritative_project(Some(&resolver), &jwt_auth(), None)
            .await
            .expect_err("ambiguous JWT must not pick a project"),
        IdentityError::ProjectContextRequired
    );
    assert_eq!(
        resolve_authoritative_project(None, &jwt_auth(), Some("project-a"))
            .await
            .expect_err("missing authority must fail closed"),
        IdentityError::AuthoritativeStateUnavailable
    );
}

#[tokio::test]
async fn api_key_ignores_conflicting_requested_project_and_keeps_its_scope() {
    let resolver = MatrixResolver::new();
    let identity =
        resolve_authoritative_project(Some(&resolver), &api_key_auth(), Some("project-b"))
            .await
            .expect("scoped API key resolves");

    assert_eq!(identity.project_id, "project-a");
    assert_eq!(
        resolver.seen.lock().expect("seen lock").as_slice(),
        &[SeenInput::ApiKey {
            api_key_id: "key-a".to_string(),
        }]
    );
}

#[tokio::test]
async fn credential_kind_does_not_depend_on_the_raw_secret_prefix() {
    let resolver = MatrixResolver::new();
    let mut auth = api_key_auth();
    auth.api_key = "jwt:api-key-secret".to_string();

    let identity = resolve_authoritative_project(Some(&resolver), &auth, Some("project-b"))
        .await
        .expect("typed API key remains pinned to its stored scope");

    assert_eq!(identity.auth_kind, AuthKind::ApiKey);
    assert_eq!(
        resolver.seen.lock().expect("seen lock").as_slice(),
        &[SeenInput::ApiKey {
            api_key_id: "key-a".to_string(),
        }]
    );
}

#[tokio::test]
async fn unscoped_api_key_is_typed_before_authority_availability() {
    let mut auth = api_key_auth();
    auth.metadata.clear();
    assert_eq!(
        resolve_authoritative_project(None, &auth, Some("project-b"))
            .await
            .expect_err("unscoped key is invalid without consulting authority"),
        IdentityError::UnscopedApiKey
    );
}
