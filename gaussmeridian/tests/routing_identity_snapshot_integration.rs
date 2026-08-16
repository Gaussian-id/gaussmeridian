use std::{
    collections::HashMap,
    convert::Infallible,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{Extension, State},
    http::{Request, StatusCode},
    middleware,
    response::Json,
    routing::post,
    Router,
};
use axum_test::TestServer;
use chrono::Utc;
use gaussmeridian_auth::{
    rate_limit::RateLimitConfig, ApiKeyManager, JwtManager, RBACManager, RateLimiter,
};
use gaussmeridian_cache::{Cache, MemoryCache, MokaL1Cache};
use gaussmeridian_core::{
    routing_policy::{
        bella::{
            BellaEstimatorPolicy, BellaLearnerState, MeridianSkillProfiler, ProfilerPolicy,
            ProfilerTrainingExample, SkillDefinition, SkillPosterior, SkillTaxonomy,
        },
        predictors::{
            FrozenPredictionEvidence, FrozenPredictionSet, PredictionEstimate,
            PredictionFeatureVector, PredictionProvenance, RouteIdentity, RoutePrediction,
        },
        r2::{
            FrozenR2Evidence, R2ActionIdentity, R2ActionPrediction, R2HeadPrediction,
            R2InstructionInputEstimate, R2Provenance, R2_INSTRUCTION_VERSION, R2_LABEL_VERSION,
            R2_PREDICTOR_VERSION, R2_RUNTIME_FEATURE_VERSION,
        },
        snapshot::RoutingInputSnapshot,
        CapabilityBand, CatalogModel, CatalogSnapshot, DeploymentKind, EvidenceSnapshot, Price,
        ProjectPolicy, RoutingBallot, TrajectoryReservationQuote,
    },
    CacheKey, CacheValue, GaussMeridian, LLMProvider, LeastConnectionsLoadBalancer,
    MeridianComplexityEstimator,
};
use gaussmeridian_db::{
    BudgetAccount, BudgetReservationRecord, BudgetReservationRepositoryTrait,
    ConfigureBudgetAccountCommand, DatabaseError, ExpireBudgetReservationCommand,
    FinalizeCommittedStreamCommand, FinalizeProviderAttemptCommand, ProviderAttemptRecord,
    ProviderAttemptRepositoryTrait, ProviderAttemptState, ProviderAttemptWriteOutcome,
    ProviderCostStatus, ReconcileProviderAttemptCostCommand, ReleaseBudgetReservationCommand,
    ReservationFinalizationOutcome, ReserveBudgetCommand, ReserveBudgetOutcome,
    SettleBudgetReservationCommand, StartProviderAttemptCommand, TransportOutcome,
};
use gaussmeridian_models::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, CompletionChunk,
    CompletionRequest, CompletionResponse, Content, CostInfo, EmbeddingRequest, EmbeddingResponse,
    Message, Model, ProviderCapabilities, ProviderConfig, ProviderError, ProviderMetadata, Role,
};
use gaussmeridian_server::{
    middleware::{
        auth_middleware_with_state, budget_reservation_middleware_with_state,
        cache_middleware_with_state, classification_middleware_with_state,
        provider_middleware_with_state, request_deadline_middleware_with_state, request_logging,
        routing_snapshot_middleware_with_state, selection_middleware_with_state, ClassificationExt,
        ExecutionRequestIdentity, ParsedGenerationExt, ParsedRequestExt, ProjectSettingsExt,
        SelectionExt,
    },
    routing::{
        attempts::{AttemptContext, AttemptLifecycle, TrajectoryIdentity},
        budget::AuthorizedBudgetReservation,
        generation::{GenerationEndpoint, GenerationRequest, GenerationTransport},
        identity::{AuthKind, IdentityError, ResolveProjectInput, ResolvedProjectIdentity},
        output_budget::{
            apply_output_budget, OutputBudgetConstraint, OUTPUT_BUDGET_INSTRUCTION_VERSION,
        },
        reconciliation::reconcile_expired_reservations,
        selection::{
            prepare_selection, with_routing_audit_headers, BallotCandidateProjection,
            SelectionAuditProjection,
        },
        snapshots::{
            PersistedRoutingInput, ResolvedGenerationRoutingInput, RoutingIdentityResolver,
            RoutingPreparationError, RoutingSnapshotPreparer,
        },
    },
    state::{AppState, RoutingConfig, RoutingMetricsData},
};
use serde_json::json;

use futures::{Stream, StreamExt};
use gaussmoa::{
    agents::{LlmAgentConfig, LlmProvider as MoaLlmProvider},
    config::{AgentConfig, AgentRole, AgentType, MoaConfig},
    ChatProvider, MoaEngine, MoaResult,
};

const JWT_SECRET: &str = "routing-identity-test-secret";
const R2_PRIMARY_OUTPUT_BUDGET: u32 = 32;
const R2_FALLBACK_OUTPUT_BUDGET: u32 = 48;
const R2_CALLER_OUTPUT_CEILING: u32 = 64;
const R2_TEST_ROUTER_COST_UPPER_BOUND: f64 = 0.001;
const R2_TEST_INPUT_PRICE_PER_MILLION: f64 = 1.0;
const R2_TEST_OUTPUT_PRICE_PER_MILLION: f64 = 2.0;

fn output_budget_constraint_fixture(output_budget: u32) -> OutputBudgetConstraint {
    let mut generation = GenerationRequest::from_http(
        "/v1/completions",
        br#"{"model":"fixture-model","prompt":"fixture"}"#,
    )
    .expect("constraint fixture parses");
    apply_output_budget(&mut generation, output_budget).expect("constraint fixture is valid")
}

fn expected_r2_reservation_amount(body: &serde_json::Value, output_budgets: [u32; 2]) -> f64 {
    let request_input_upper_bound = u32::try_from(serde_json::to_vec(body).unwrap().len()).unwrap();
    R2_TEST_ROUTER_COST_UPPER_BOUND
        + output_budgets
            .into_iter()
            .map(|output_budget| {
                let constraint = output_budget_constraint_fixture(output_budget);
                (f64::from(request_input_upper_bound + constraint.input_token_upper_bound)
                    * R2_TEST_INPUT_PRICE_PER_MILLION
                    + f64::from(output_budget) * R2_TEST_OUTPUT_PRICE_PER_MILLION)
                    / 1_000_000.0
            })
            .sum::<f64>()
}

#[derive(Clone)]
struct StubIdentityResolver {
    result: Result<ResolvedProjectIdentity, IdentityError>,
}

#[derive(Clone)]
struct RequestedProjectResolver;

#[derive(Clone)]
struct UnavailableSnapshotPreparer;

#[derive(Clone)]
struct SuccessfulSnapshotPreparer {
    prepared_projects: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone)]
struct BallotSnapshotPreparer {
    catalog: CatalogSnapshot,
}

#[derive(Clone)]
struct LearnedBallotSnapshotPreparer {
    catalog: CatalogSnapshot,
}

#[derive(Clone)]
struct R2BallotSnapshotPreparer {
    catalog: CatalogSnapshot,
    output_budgets: [u32; 2],
}

#[derive(Clone)]
enum ReservationBehavior {
    Reserve,
    ReserveThenReplay,
    ReserveThenConflict,
    Insufficient,
    Unavailable,
    SettlementUnavailable,
}

struct RecordingBudgetRepository {
    behavior: ReservationBehavior,
    commands: Arc<Mutex<Vec<ReserveBudgetCommand>>>,
    committed_attempts: Option<Arc<RecordingAttemptRepository>>,
}

#[async_trait]
impl BudgetReservationRepositoryTrait for RecordingBudgetRepository {
    async fn configure_account(
        &self,
        _command: ConfigureBudgetAccountCommand,
    ) -> Result<BudgetAccount, DatabaseError> {
        Err(DatabaseError::Other("not exercised".to_string()))
    }

    async fn reserve(
        &self,
        command: ReserveBudgetCommand,
    ) -> Result<ReserveBudgetOutcome, DatabaseError> {
        let previous = {
            let mut commands = self.commands.lock().unwrap();
            let previous = commands.first().cloned();
            commands.push(command.clone());
            previous
        };
        match self.behavior {
            ReservationBehavior::Reserve | ReservationBehavior::SettlementUnavailable => {
                Ok(ReserveBudgetOutcome::Reserved(reservation_record(command)))
            }
            ReservationBehavior::ReserveThenReplay => match previous {
                Some(previous) => Ok(ReserveBudgetOutcome::AlreadyReserved(reservation_record(
                    previous,
                ))),
                None => Ok(ReserveBudgetOutcome::Reserved(reservation_record(command))),
            },
            ReservationBehavior::ReserveThenConflict => match previous {
                Some(previous) => Ok(ReserveBudgetOutcome::IdempotencyConflict {
                    existing: reservation_record(previous),
                }),
                None => Ok(ReserveBudgetOutcome::Reserved(reservation_record(command))),
            },
            ReservationBehavior::Insufficient => Ok(ReserveBudgetOutcome::InsufficientBudget {
                requested: command.amount,
                available: 0.0,
            }),
            ReservationBehavior::Unavailable => {
                Err(DatabaseError::Other("budget store unavailable".to_string()))
            }
        }
    }

    async fn settle(
        &self,
        command: SettleBudgetReservationCommand,
    ) -> Result<ReservationFinalizationOutcome, DatabaseError> {
        if matches!(self.behavior, ReservationBehavior::SettlementUnavailable) {
            return Err(DatabaseError::Other(
                "injected settlement failure".to_string(),
            ));
        }
        let mut record = self
            .commands
            .lock()
            .unwrap()
            .iter()
            .find(|candidate| candidate.reservation_id == command.reservation_id)
            .cloned()
            .map(reservation_record)
            .ok_or_else(|| DatabaseError::NotFound("test reservation".to_string()))?;
        if command.terminal.response_committed {
            let attempt_id = command
                .terminal
                .committed_attempt_id
                .as_deref()
                .ok_or_else(|| {
                    DatabaseError::InvalidData("missing committed attempt".to_string())
                })?;
            if let Some(attempts) = &self.committed_attempts {
                attempts.commit_response(attempt_id, &command.finalization_id)?;
            }
        }
        record.state = "settled".to_string();
        record.actual_provider_cost = Some(command.actual_provider_cost);
        record.finalization_id = Some(command.finalization_id);
        record.terminal_outcome = Some(command.terminal.outcome.as_str().to_string());
        record.response_committed = Some(command.terminal.response_committed);
        record.committed_attempt_id = command.terminal.committed_attempt_id;
        record.customer_charge = Some(command.terminal.customer_charge);
        record.finalized_at = Some(command.finalized_at);
        Ok(ReservationFinalizationOutcome::Finalized(record))
    }

    async fn finalize_committed_stream(
        &self,
        command: FinalizeCommittedStreamCommand,
    ) -> Result<ReservationFinalizationOutcome, DatabaseError> {
        let attempts = self
            .committed_attempts
            .as_ref()
            .ok_or_else(|| DatabaseError::Other("attempt store unavailable".to_string()))?;
        *attempts.atomic_finalizations.lock().unwrap() += 1;
        if matches!(self.behavior, ReservationBehavior::SettlementUnavailable) {
            return Err(DatabaseError::Other(
                "injected settlement failure".to_string(),
            ));
        }
        attempts.finalize(command.attempt.clone()).await?;
        let actual_provider_cost = attempts
            .total_provider_cost(&command.reservation_id)
            .await?;
        let mut record = self
            .commands
            .lock()
            .unwrap()
            .iter()
            .find(|candidate| candidate.reservation_id == command.reservation_id)
            .cloned()
            .map(reservation_record)
            .ok_or_else(|| DatabaseError::NotFound("test reservation".to_string()))?;
        record.state = "settled".to_string();
        record.actual_provider_cost = Some(actual_provider_cost);
        record.finalization_id = Some(command.attempt.finalization_id);
        record.terminal_outcome = Some(command.terminal.outcome.as_str().to_string());
        record.response_committed = Some(true);
        record.committed_attempt_id = command.terminal.committed_attempt_id;
        record.customer_charge = Some(command.terminal.customer_charge);
        record.finalized_at = Some(command.attempt.finalized_at);
        Ok(ReservationFinalizationOutcome::Finalized(record))
    }

    async fn release(
        &self,
        command: ReleaseBudgetReservationCommand,
    ) -> Result<ReservationFinalizationOutcome, DatabaseError> {
        let mut record = self
            .commands
            .lock()
            .unwrap()
            .iter()
            .find(|candidate| candidate.reservation_id == command.reservation_id)
            .cloned()
            .map(reservation_record)
            .ok_or_else(|| DatabaseError::NotFound("test reservation".to_string()))?;
        record.state = "released".to_string();
        record.actual_provider_cost = Some(0.0);
        record.finalization_id = Some(command.finalization_id);
        record.terminal_outcome = Some(command.terminal.outcome.as_str().to_string());
        record.response_committed = Some(command.terminal.response_committed);
        record.committed_attempt_id = command.terminal.committed_attempt_id;
        record.customer_charge = Some(command.terminal.customer_charge);
        record.finalized_at = Some(command.finalized_at);
        Ok(ReservationFinalizationOutcome::Finalized(record))
    }

    async fn expire(
        &self,
        command: ExpireBudgetReservationCommand,
    ) -> Result<ReservationFinalizationOutcome, DatabaseError> {
        let mut record = self
            .commands
            .lock()
            .unwrap()
            .iter()
            .find(|candidate| candidate.reservation_id == command.reservation_id)
            .cloned()
            .map(reservation_record)
            .ok_or_else(|| DatabaseError::NotFound(command.reservation_id.clone()))?;
        record.state = "expired".to_string();
        record.actual_provider_cost = Some(command.actual_provider_cost);
        record.finalization_id = Some(command.finalization_id);
        record.terminal_outcome = Some(command.terminal.outcome.as_str().to_string());
        record.response_committed = Some(command.terminal.response_committed);
        record.committed_attempt_id = command.terminal.committed_attempt_id;
        record.customer_charge = Some(command.terminal.customer_charge);
        record.finalized_at = Some(command.observed_at);
        Ok(ReservationFinalizationOutcome::Finalized(record))
    }

    async fn get_account(
        &self,
        _project_id: &str,
        _period_key: &str,
    ) -> Result<Option<BudgetAccount>, DatabaseError> {
        Err(DatabaseError::Other("not exercised".to_string()))
    }

    async fn get_reservation(
        &self,
        reservation_id: &str,
    ) -> Result<Option<BudgetReservationRecord>, DatabaseError> {
        Ok(self
            .commands
            .lock()
            .unwrap()
            .iter()
            .find(|command| command.reservation_id == reservation_id)
            .cloned()
            .map(reservation_record))
    }

    async fn list_expired_active(
        &self,
        observed_at: chrono::DateTime<chrono::Utc>,
        limit: u32,
    ) -> Result<Vec<BudgetReservationRecord>, DatabaseError> {
        Ok(self
            .commands
            .lock()
            .unwrap()
            .iter()
            .filter(|command| command.expires_at <= observed_at)
            .take(limit as usize)
            .cloned()
            .map(reservation_record)
            .collect())
    }
}

#[derive(Default)]
struct RecordingAttemptRepository {
    records: Mutex<Vec<ProviderAttemptRecord>>,
    finalizations: Mutex<HashMap<String, u32>>,
    atomic_finalizations: Mutex<u32>,
    fail_finalization: bool,
}

impl RecordingAttemptRepository {
    fn finalization_count(&self, attempt_id: &str) -> u32 {
        self.finalizations
            .lock()
            .unwrap()
            .get(attempt_id)
            .copied()
            .unwrap_or_default()
    }

    fn atomic_finalization_count(&self) -> u32 {
        *self.atomic_finalizations.lock().unwrap()
    }

    fn commit_response(
        &self,
        attempt_id: &str,
        finalization_id: &str,
    ) -> Result<(), DatabaseError> {
        let mut records = self.records.lock().unwrap();
        let record = records
            .iter_mut()
            .find(|record| record.attempt_id == attempt_id)
            .ok_or_else(|| DatabaseError::NotFound(attempt_id.to_string()))?;
        if record.state == ProviderAttemptState::Started
            || record.finalization_id.as_deref() != Some(finalization_id)
        {
            return Err(DatabaseError::InvalidData(
                "attempt is not eligible for atomic response commitment".to_string(),
            ));
        }
        record.response_committed = true;
        Ok(())
    }
}

#[async_trait]
impl ProviderAttemptRepositoryTrait for RecordingAttemptRepository {
    async fn start(
        &self,
        command: StartProviderAttemptCommand,
    ) -> Result<ProviderAttemptWriteOutcome, DatabaseError> {
        let record = ProviderAttemptRecord {
            attempt_id: command.attempt_id,
            reservation_id: Some(command.reservation_id),
            request_id: command.request_id,
            project_id: Some(command.project_id),
            snapshot_fingerprint: command.snapshot_fingerprint,
            ballot_id: command.ballot_id,
            attempt_no: command.attempt_no,
            provider_id: command.provider_id,
            model_id: command.model_id,
            requested_output_tokens: command.requested_output_tokens,
            actual_output_tokens: None,
            finish_reason: None,
            output_constraint_version: command.output_constraint_version,
            token_limit_compliant: None,
            state: ProviderAttemptState::Started,
            finalization_id: None,
            transport_outcome: None,
            response_committed: false,
            input_cost: 0.0,
            output_cost: 0.0,
            reasoning_cost: 0.0,
            tools_cost: 0.0,
            other_cost: 0.0,
            provider_cost_incurred: 0.0,
            cost_status: ProviderCostStatus::Pending,
            cost_reconciliation_id: None,
            cost_reconciled_at: None,
            error_code: None,
            price_version: command.price_version,
            catalog_version: command.catalog_version,
            policy_version: command.policy_version,
            model_version: command.model_version,
            started_at: command.started_at,
            finalized_at: None,
        };
        self.records.lock().unwrap().push(record.clone());
        Ok(ProviderAttemptWriteOutcome::Created(record))
    }

    async fn finalize(
        &self,
        command: FinalizeProviderAttemptCommand,
    ) -> Result<ProviderAttemptWriteOutcome, DatabaseError> {
        *self
            .finalizations
            .lock()
            .unwrap()
            .entry(command.attempt_id.clone())
            .or_default() += 1;
        if self.fail_finalization {
            return Err(DatabaseError::Other(
                "injected attempt finalization failure".to_string(),
            ));
        }
        let mut records = self.records.lock().unwrap();
        let record = records
            .iter_mut()
            .find(|record| record.attempt_id == command.attempt_id)
            .ok_or_else(|| DatabaseError::NotFound(command.attempt_id.clone()))?;
        if record.output_constraint_version != command.output_constraint_version {
            return Err(DatabaseError::IdempotencyConflict {
                entity: "provider_attempt_output_constraint",
                key: command.attempt_id,
            });
        }
        record.state = match command.terminal_state {
            gaussmeridian_db::ProviderAttemptTerminalState::Succeeded => {
                ProviderAttemptState::Succeeded
            }
            gaussmeridian_db::ProviderAttemptTerminalState::Failed => ProviderAttemptState::Failed,
            gaussmeridian_db::ProviderAttemptTerminalState::TimedOut => {
                ProviderAttemptState::TimedOut
            }
            gaussmeridian_db::ProviderAttemptTerminalState::Cancelled => {
                ProviderAttemptState::Cancelled
            }
        };
        record.finalization_id = Some(command.finalization_id);
        record.transport_outcome = Some(command.transport_outcome);
        record.response_committed = command.response_committed;
        record.actual_output_tokens = command.actual_output_tokens;
        record.finish_reason = command.finish_reason;
        record.token_limit_compliant = command.token_limit_compliant;
        record.input_cost = command.input_cost;
        record.output_cost = command.output_cost;
        record.reasoning_cost = command.reasoning_cost;
        record.tools_cost = command.tools_cost;
        record.other_cost = command.other_cost;
        record.provider_cost_incurred = command.input_cost
            + command.output_cost
            + command.reasoning_cost
            + command.tools_cost
            + command.other_cost;
        record.cost_status = command.cost_status;
        record.error_code = command.error_code;
        record.finalized_at = Some(command.finalized_at);
        Ok(ProviderAttemptWriteOutcome::Finalized(record.clone()))
    }

    async fn reconcile_cost(
        &self,
        _command: ReconcileProviderAttemptCostCommand,
    ) -> Result<ProviderAttemptWriteOutcome, DatabaseError> {
        Err(DatabaseError::Other("not exercised".to_string()))
    }

    async fn list_for_reservation(
        &self,
        reservation_id: &str,
    ) -> Result<Vec<ProviderAttemptRecord>, DatabaseError> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| record.reservation_id.as_deref() == Some(reservation_id))
            .cloned()
            .collect())
    }

    async fn total_provider_cost(&self, reservation_id: &str) -> Result<f64, DatabaseError> {
        let records = self.list_for_reservation(reservation_id).await?;
        if records.iter().any(|record| {
            matches!(
                record.cost_status,
                ProviderCostStatus::Pending | ProviderCostStatus::Unresolved
            )
        }) {
            return Err(DatabaseError::ProviderLiabilityUnresolved {
                reservation_id: reservation_id.to_string(),
            });
        }
        Ok(records
            .iter()
            .map(|record| record.provider_cost_incurred)
            .sum())
    }
}

struct BlockingAttemptRepository {
    inner: Arc<RecordingAttemptRepository>,
    block_succeeded_once: AtomicBool,
    calls: AtomicU32,
    succeeded_commands: Mutex<Vec<FinalizeProviderAttemptCommand>>,
    blocked: tokio::sync::Notify,
}

impl BlockingAttemptRepository {
    fn new(inner: Arc<RecordingAttemptRepository>) -> Self {
        Self {
            inner,
            block_succeeded_once: AtomicBool::new(true),
            calls: AtomicU32::new(0),
            succeeded_commands: Mutex::new(Vec::new()),
            blocked: tokio::sync::Notify::new(),
        }
    }
}

#[async_trait]
impl ProviderAttemptRepositoryTrait for BlockingAttemptRepository {
    async fn start(
        &self,
        command: StartProviderAttemptCommand,
    ) -> Result<ProviderAttemptWriteOutcome, DatabaseError> {
        self.inner.start(command).await
    }

    async fn finalize(
        &self,
        command: FinalizeProviderAttemptCommand,
    ) -> Result<ProviderAttemptWriteOutcome, DatabaseError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let succeeded = matches!(
            command.terminal_state,
            gaussmeridian_db::ProviderAttemptTerminalState::Succeeded
        );
        if succeeded {
            self.succeeded_commands
                .lock()
                .unwrap()
                .push(command.clone());
        }
        if succeeded && self.block_succeeded_once.swap(false, Ordering::SeqCst) {
            self.blocked.notify_one();
            futures::future::pending::<()>().await;
        }
        self.inner.finalize(command).await
    }

    async fn reconcile_cost(
        &self,
        command: ReconcileProviderAttemptCostCommand,
    ) -> Result<ProviderAttemptWriteOutcome, DatabaseError> {
        self.inner.reconcile_cost(command).await
    }

    async fn list_for_reservation(
        &self,
        reservation_id: &str,
    ) -> Result<Vec<ProviderAttemptRecord>, DatabaseError> {
        self.inner.list_for_reservation(reservation_id).await
    }

    async fn total_provider_cost(&self, reservation_id: &str) -> Result<f64, DatabaseError> {
        self.inner.total_provider_cost(reservation_id).await
    }
}

struct BlockingCommittedStreamRepository {
    inner: Arc<RecordingBudgetRepository>,
    block_once: AtomicBool,
    calls: AtomicU32,
    commands: Mutex<Vec<FinalizeCommittedStreamCommand>>,
    completed_reservations: Mutex<Vec<BudgetReservationRecord>>,
    blocked: tokio::sync::Notify,
}

impl BlockingCommittedStreamRepository {
    fn new(inner: Arc<RecordingBudgetRepository>) -> Self {
        Self {
            inner,
            block_once: AtomicBool::new(true),
            calls: AtomicU32::new(0),
            commands: Mutex::new(Vec::new()),
            completed_reservations: Mutex::new(Vec::new()),
            blocked: tokio::sync::Notify::new(),
        }
    }
}

#[async_trait]
impl BudgetReservationRepositoryTrait for BlockingCommittedStreamRepository {
    async fn configure_account(
        &self,
        command: ConfigureBudgetAccountCommand,
    ) -> Result<BudgetAccount, DatabaseError> {
        self.inner.configure_account(command).await
    }

    async fn reserve(
        &self,
        command: ReserveBudgetCommand,
    ) -> Result<ReserveBudgetOutcome, DatabaseError> {
        self.inner.reserve(command).await
    }

    async fn settle(
        &self,
        command: SettleBudgetReservationCommand,
    ) -> Result<ReservationFinalizationOutcome, DatabaseError> {
        self.inner.settle(command).await
    }

    async fn finalize_committed_stream(
        &self,
        command: FinalizeCommittedStreamCommand,
    ) -> Result<ReservationFinalizationOutcome, DatabaseError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.commands.lock().unwrap().push(command.clone());
        if self.block_once.swap(false, Ordering::SeqCst) {
            self.blocked.notify_one();
            futures::future::pending::<()>().await;
        }
        let outcome = self.inner.finalize_committed_stream(command).await?;
        match &outcome {
            ReservationFinalizationOutcome::Finalized(record)
            | ReservationFinalizationOutcome::AlreadyFinalized(record) => self
                .completed_reservations
                .lock()
                .unwrap()
                .push(record.clone()),
            ReservationFinalizationOutcome::NotYetExpired { .. } => {}
        }
        Ok(outcome)
    }

    async fn release(
        &self,
        command: ReleaseBudgetReservationCommand,
    ) -> Result<ReservationFinalizationOutcome, DatabaseError> {
        self.inner.release(command).await
    }

    async fn expire(
        &self,
        command: ExpireBudgetReservationCommand,
    ) -> Result<ReservationFinalizationOutcome, DatabaseError> {
        self.inner.expire(command).await
    }

    async fn get_reservation(
        &self,
        reservation_id: &str,
    ) -> Result<Option<BudgetReservationRecord>, DatabaseError> {
        self.inner.get_reservation(reservation_id).await
    }

    async fn list_expired_active(
        &self,
        observed_at: chrono::DateTime<chrono::Utc>,
        limit: u32,
    ) -> Result<Vec<BudgetReservationRecord>, DatabaseError> {
        self.inner.list_expired_active(observed_at, limit).await
    }

    async fn get_account(
        &self,
        project_id: &str,
        period_key: &str,
    ) -> Result<Option<BudgetAccount>, DatabaseError> {
        self.inner.get_account(project_id, period_key).await
    }
}

fn assert_attempt_matches_finalization(
    record: &ProviderAttemptRecord,
    command: &FinalizeProviderAttemptCommand,
) {
    let expected_state = match command.terminal_state {
        gaussmeridian_db::ProviderAttemptTerminalState::Succeeded => {
            ProviderAttemptState::Succeeded
        }
        gaussmeridian_db::ProviderAttemptTerminalState::Failed => ProviderAttemptState::Failed,
        gaussmeridian_db::ProviderAttemptTerminalState::TimedOut => ProviderAttemptState::TimedOut,
        gaussmeridian_db::ProviderAttemptTerminalState::Cancelled => {
            ProviderAttemptState::Cancelled
        }
    };
    assert_eq!(record.attempt_id, command.attempt_id);
    assert_eq!(record.state, expected_state);
    assert_eq!(
        record.finalization_id.as_deref(),
        Some(command.finalization_id.as_str())
    );
    assert_eq!(record.transport_outcome, Some(command.transport_outcome));
    assert_eq!(record.response_committed, command.response_committed);
    assert_eq!(record.actual_output_tokens, command.actual_output_tokens);
    assert_eq!(record.finish_reason, command.finish_reason);
    assert_eq!(
        record.output_constraint_version,
        command.output_constraint_version
    );
    assert_eq!(record.token_limit_compliant, command.token_limit_compliant);
    assert_eq!(record.cost_status, command.cost_status);
    assert_eq!(record.input_cost, command.input_cost);
    assert_eq!(record.output_cost, command.output_cost);
    assert_eq!(record.reasoning_cost, command.reasoning_cost);
    assert_eq!(record.tools_cost, command.tools_cost);
    assert_eq!(record.other_cost, command.other_cost);
    assert_eq!(
        record.provider_cost_incurred,
        command.input_cost
            + command.output_cost
            + command.reasoning_cost
            + command.tools_cost
            + command.other_cost
    );
    assert_eq!(record.error_code, command.error_code);
    assert_eq!(record.finalized_at, Some(command.finalized_at));
}

fn reservation_record(command: ReserveBudgetCommand) -> BudgetReservationRecord {
    BudgetReservationRecord {
        reservation_id: command.reservation_id,
        request_id: command.request_id,
        idempotency_key: command.idempotency_key,
        project_id: Some(command.project_id),
        snapshot_fingerprint: command.snapshot_fingerprint,
        period_key: command.period_key,
        amount: command.amount,
        state: "active".to_string(),
        actual_provider_cost: None,
        finalization_id: None,
        terminal_outcome: None,
        response_committed: None,
        committed_attempt_id: None,
        customer_charge: None,
        finalized_at: None,
        expires_at: command.expires_at,
        created_at: command.created_at,
    }
}

#[async_trait]
impl RoutingSnapshotPreparer for UnavailableSnapshotPreparer {
    async fn prepare(
        &self,
        _input: ResolvedGenerationRoutingInput,
    ) -> Result<PersistedRoutingInput, RoutingPreparationError> {
        Err(RoutingPreparationError::Policy(
            gaussmeridian_server::routing::snapshots::PolicyLoadError::Unavailable,
        ))
    }
}

#[async_trait]
impl RoutingSnapshotPreparer for SuccessfulSnapshotPreparer {
    async fn prepare(
        &self,
        input: ResolvedGenerationRoutingInput,
    ) -> Result<PersistedRoutingInput, RoutingPreparationError> {
        let snapshot = RoutingInputSnapshot {
            schema_version: "routing-input/v1".to_string(),
            project_id: input.identity.project_id.clone(),
            request: input.request,
            policy: ProjectPolicy {
                cost_weight: 0.2,
                quality_floor: 0.7,
                max_band: gaussmeridian_core::routing_policy::CapabilityBand::Frontier,
                moderate_complexity_threshold: 0.35,
                high_complexity_threshold: 0.7,
                max_provider_attempts: 3,
                router_cost_upper_bound: 0.001,
            },
            catalog: CatalogSnapshot::new(Vec::new()),
            evidence: EvidenceSnapshot {
                policy_version: "policy-test-v1".to_string(),
                catalog_version: "catalog-test-v1".to_string(),
                price_version: "price-test-v1".to_string(),
                evaluator_version: "evaluator-test-v1".to_string(),
                normalized_cost_floor: 0.0,
                normalized_cost_ceiling: 1.0,
                complexity: None,
                predictions: Default::default(),
                bella: Default::default(),
                r2: Default::default(),
                compound: Default::default(),
            },
            feature_version: "feature-test-v1".to_string(),
            degradations: Vec::new(),
        };
        let frozen = snapshot.freeze()?;
        self.prepared_projects
            .lock()
            .unwrap()
            .push(input.identity.project_id.clone());
        Ok(PersistedRoutingInput {
            identity: input.identity,
            snapshot,
            frozen,
        })
    }
}

#[async_trait]
impl RoutingSnapshotPreparer for BallotSnapshotPreparer {
    async fn prepare(
        &self,
        input: ResolvedGenerationRoutingInput,
    ) -> Result<PersistedRoutingInput, RoutingPreparationError> {
        let snapshot = RoutingInputSnapshot {
            schema_version: "routing-input/v3".to_string(),
            project_id: input.identity.project_id.clone(),
            request: input.request,
            policy: ProjectPolicy {
                cost_weight: 0.2,
                quality_floor: 0.7,
                max_band: CapabilityBand::Frontier,
                moderate_complexity_threshold: 0.35,
                high_complexity_threshold: 0.7,
                max_provider_attempts: 2,
                router_cost_upper_bound: 0.001,
            },
            catalog: self.catalog.clone(),
            evidence: EvidenceSnapshot {
                policy_version: "policy-test-v2".to_string(),
                catalog_version: "catalog-test-v2".to_string(),
                price_version: "price-test-v2".to_string(),
                evaluator_version: "evaluator-test-v2".to_string(),
                normalized_cost_floor: 0.0,
                normalized_cost_ceiling: 1.0,
                complexity: None,
                predictions: Default::default(),
                bella: Default::default(),
                r2: Default::default(),
                compound: Default::default(),
            },
            feature_version: "feature-test-v2".to_string(),
            degradations: Vec::new(),
        };
        let frozen = snapshot.freeze()?;
        Ok(PersistedRoutingInput {
            identity: input.identity,
            snapshot,
            frozen,
        })
    }
}

#[async_trait]
impl RoutingSnapshotPreparer for LearnedBallotSnapshotPreparer {
    async fn prepare(
        &self,
        input: ResolvedGenerationRoutingInput,
    ) -> Result<PersistedRoutingInput, RoutingPreparationError> {
        let feature_version = "carrot-runtime-features/v1";
        let features = PredictionFeatureVector::new(
            feature_version,
            vec![
                f64::from(input.request.complexity),
                f64::from(input.request.estimated_input_tokens).ln_1p(),
                f64::from(input.request.output_token_budget).ln_1p(),
            ],
        )
        .expect("learned test features are valid");
        let predictions = self
            .catalog
            .models()
            .iter()
            .map(|model| {
                let route = RouteIdentity::new(&model.provider_id, &model.model_id)
                    .expect("learned test route is valid");
                let (correctness, cost) = match model.model_id.as_str() {
                    "forbidden-model" => (1.0, 0.0),
                    "learned-first" => (0.98, 0.02),
                    _ => (0.75, 0.01),
                };
                RoutePrediction::new(
                    route,
                    PredictionEstimate::estimated(correctness, 0.95)
                        .expect("learned outcome is valid"),
                    PredictionEstimate::expected_cost(cost, 0.95).expect("learned cost is valid"),
                )
                .expect("learned route prediction is valid")
            })
            .collect();
        let predictions = FrozenPredictionSet::new(
            PredictionProvenance::new(
                "carrot-knn/v1",
                feature_version,
                "evaluator-test-v2",
                "corpus-test-v2",
                "catalog-test-v2",
                "price-test-v2",
                "learner-state-active-test",
                "training-content-active-test",
            )
            .expect("learned provenance is valid"),
            features,
            predictions,
        )
        .expect("learned prediction set is valid");
        let bella = learned_bella_learner().freeze(
            &input.task_text,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "catalog-test-v2",
        );
        let snapshot = RoutingInputSnapshot {
            schema_version: "routing-input/v6".to_string(),
            project_id: input.identity.project_id.clone(),
            request: input.request,
            policy: ProjectPolicy {
                cost_weight: 0.2,
                quality_floor: 0.7,
                max_band: CapabilityBand::Frontier,
                moderate_complexity_threshold: 0.35,
                high_complexity_threshold: 0.7,
                max_provider_attempts: 2,
                router_cost_upper_bound: 0.001,
            },
            catalog: self.catalog.clone(),
            evidence: EvidenceSnapshot {
                policy_version: "policy-test-v2".to_string(),
                catalog_version: "catalog-test-v2".to_string(),
                price_version: "price-test-v2".to_string(),
                evaluator_version: "evaluator-test-v2".to_string(),
                normalized_cost_floor: 0.0,
                normalized_cost_ceiling: 1.0,
                complexity: Some(input.complexity_evidence),
                predictions: FrozenPredictionEvidence::Active(predictions),
                bella,
                r2: Default::default(),
                compound: Default::default(),
            },
            feature_version: feature_version.to_string(),
            degradations: Vec::new(),
        };
        let frozen = snapshot.freeze()?;
        Ok(PersistedRoutingInput {
            identity: input.identity,
            snapshot,
            frozen,
        })
    }
}

#[async_trait]
impl RoutingSnapshotPreparer for R2BallotSnapshotPreparer {
    async fn prepare(
        &self,
        input: ResolvedGenerationRoutingInput,
    ) -> Result<PersistedRoutingInput, RoutingPreparationError> {
        let predecessor_constraint =
            output_budget_constraint_fixture(input.request.output_token_budget);
        let provenance = R2Provenance::new(
            R2_PREDICTOR_VERSION,
            "meridian-r2-encoder/v1",
            R2_RUNTIME_FEATURE_VERSION,
            "evaluator-r2-provider-test",
            "corpus-r2-provider-test",
            "catalog-r2-provider-test",
            "price-r2-provider-test",
            R2_INSTRUCTION_VERSION,
            R2_LABEL_VERSION,
            "a".repeat(64),
            "b".repeat(64),
        )
        .expect("R2 provider provenance is valid");
        let predictions = self
            .catalog
            .models()
            .iter()
            .zip(self.output_budgets.iter().copied())
            .map(|(model, selected_budget)| {
                let selected_constraint = output_budget_constraint_fixture(selected_budget);
                R2HeadPrediction::Estimated(R2ActionPrediction {
                    action: R2ActionIdentity::new(
                        RouteIdentity::new(&model.provider_id, &model.model_id)
                            .expect("provider route is valid"),
                        selected_budget,
                    )
                    .expect("selected budget is valid"),
                    semantic_correctness: PredictionEstimate::estimated(0.99, 0.95)
                        .expect("semantic estimate is valid"),
                    expected_output_tokens: PredictionEstimate::expected_cost(16.0, 0.95)
                        .expect("output estimate is valid"),
                    instruction_input_tokens: selected_constraint.estimated_input_tokens,
                    instruction_input_upper_bound: selected_constraint.input_token_upper_bound,
                })
            })
            .collect();
        let r2 = FrozenR2Evidence::active(
            provenance,
            R2InstructionInputEstimate::new(
                predecessor_constraint.estimated_input_tokens,
                predecessor_constraint.input_token_upper_bound,
            )
            .expect("predecessor instruction estimate is valid"),
            predictions,
        )
        .expect("R2 provider evidence is valid");
        let snapshot = RoutingInputSnapshot {
            schema_version: "routing-input/v7".to_string(),
            project_id: input.identity.project_id.clone(),
            request: input.request,
            policy: ProjectPolicy {
                cost_weight: 0.2,
                quality_floor: 0.7,
                max_band: CapabilityBand::Frontier,
                moderate_complexity_threshold: 0.35,
                high_complexity_threshold: 0.7,
                max_provider_attempts: 2,
                router_cost_upper_bound: 0.001,
            },
            catalog: self.catalog.clone(),
            evidence: EvidenceSnapshot {
                policy_version: "policy-r2-provider-test".to_string(),
                catalog_version: "catalog-r2-provider-test".to_string(),
                price_version: "price-r2-provider-test".to_string(),
                evaluator_version: "evaluator-r2-provider-test".to_string(),
                normalized_cost_floor: 0.0,
                normalized_cost_ceiling: 1.0,
                complexity: Some(input.complexity_evidence),
                predictions: Default::default(),
                bella: Default::default(),
                r2,
                compound: Default::default(),
            },
            feature_version: R2_RUNTIME_FEATURE_VERSION.to_string(),
            degradations: Vec::new(),
        };
        let frozen = snapshot.freeze()?;
        Ok(PersistedRoutingInput {
            identity: input.identity,
            snapshot,
            frozen,
        })
    }
}

fn catalog_model(model_id: &str, provider_id: &str, compliant: bool) -> CatalogModel {
    CatalogModel {
        model_id: model_id.to_string(),
        provider_id: provider_id.to_string(),
        model_version: "model-v1".to_string(),
        capability_band: CapabilityBand::Advanced,
        deployment_kind: DeploymentKind::Managed,
        price: Price {
            input_per_million: 1.0,
            output_per_million: 2.0,
            expected_fixed_cost: 0.0,
            fixed_cost_upper_bound: 0.0,
        },
        semantic_quality_prior: 0.9,
        transport_success_probability: 0.99,
        credential_available: true,
        adapter_registered: true,
        adapter_supports_model: true,
        tenant_allowed: true,
        compliant,
        skill_proficiency: [1.0; 12],
    }
}

fn learned_catalog_model(model_id: &str, provider_id: &str, compliant: bool) -> CatalogModel {
    let mut model = catalog_model(model_id, provider_id, compliant);
    model.price.fixed_cost_upper_bound = 1.0;
    model
}

fn learned_bella_learner() -> BellaLearnerState {
    let taxonomy = SkillTaxonomy::new(
        "skills-test/v1",
        vec![SkillDefinition::new(
            0,
            "code_synthesis",
            "Code synthesis",
            "Implement and compile source code.",
            0.6,
        )
        .expect("BELLA test skill is valid")],
    )
    .expect("BELLA test taxonomy is valid");
    let profiler = MeridianSkillProfiler::train(
        &taxonomy,
        vec![ProfilerTrainingExample::new(
            "code-test",
            "Implement and compile Rust code",
            vec!["code_synthesis".to_string()],
        )
        .expect("BELLA profiler row is valid")],
        ProfilerPolicy::new(0.3, 0.05, 5).expect("BELLA profiler policy is valid"),
    )
    .expect("BELLA test profiler trains");
    let posteriors = [
        ("provider-b", "learned-first"),
        ("provider-c", "learned-second"),
    ]
    .into_iter()
    .map(|(provider_id, model_id)| {
        SkillPosterior::new(
            RouteIdentity::new(provider_id, model_id).expect("BELLA test route is valid"),
            "skills-test/v1",
            "code_synthesis",
            1.0,
            1.0,
            20,
            0,
        )
        .expect("BELLA test posterior is valid")
    })
    .collect();

    BellaLearnerState::new(
        taxonomy,
        profiler,
        "controlled-skill-critic-test/v1",
        "evaluator-test-v2",
        "bella-corpus-test/v1",
        "catalog-test-v2",
        BellaEstimatorPolicy::new(3, 0.0).expect("BELLA estimator policy is valid"),
        posteriors,
    )
    .expect("BELLA test learner is valid")
}

fn persisted_ballot_input(max_provider_attempts: u32) -> PersistedRoutingInput {
    let identity = ResolvedProjectIdentity {
        project_id: "project-a".to_string(),
        org_id: "org-a".to_string(),
        principal_id: "user-a".to_string(),
        auth_kind: AuthKind::Jwt,
    };
    let snapshot = RoutingInputSnapshot {
        schema_version: "routing-input/v3".to_string(),
        project_id: identity.project_id.clone(),
        request: gaussmeridian_core::routing_policy::RoutingContext {
            complexity: 0.9,
            estimated_input_tokens: 32,
            input_token_upper_bound: 64,
            output_token_budget: 64,
            hard_skills: Vec::new(),
        },
        policy: ProjectPolicy {
            cost_weight: 0.2,
            quality_floor: 0.7,
            max_band: CapabilityBand::Frontier,
            moderate_complexity_threshold: 0.35,
            high_complexity_threshold: 0.7,
            max_provider_attempts,
            router_cost_upper_bound: 0.001,
        },
        catalog: CatalogSnapshot::new(vec![
            catalog_model("eligible-model", "provider-b", true),
            catalog_model("eligible-model-2", "provider-c", true),
            catalog_model("eligible-model-3", "provider-d", true),
        ]),
        evidence: EvidenceSnapshot {
            policy_version: "policy-provider-test".to_string(),
            catalog_version: "catalog-provider-test".to_string(),
            price_version: "price-provider-test".to_string(),
            evaluator_version: "evaluator-provider-test".to_string(),
            normalized_cost_floor: 0.0,
            normalized_cost_ceiling: 1.0,
            complexity: None,
            predictions: Default::default(),
            bella: Default::default(),
            r2: Default::default(),
            compound: Default::default(),
        },
        feature_version: "feature-provider-test".to_string(),
        degradations: Vec::new(),
    };
    let frozen = snapshot.freeze().expect("provider test snapshot freezes");
    PersistedRoutingInput {
        identity,
        snapshot,
        frozen,
    }
}

#[test]
fn prepared_selection_projects_durable_band_and_version_evidence() {
    let persisted = persisted_ballot_input(2);
    let prepared = prepare_selection(&persisted).expect("persisted input selects");

    assert_eq!(
        prepared.audit.band_decision.desired,
        CapabilityBand::Frontier
    );
    assert_eq!(
        prepared.audit.band_decision.selected,
        CapabilityBand::Advanced
    );
    assert_eq!(
        serde_json::to_value(prepared.audit.band_decision.reason).unwrap(),
        json!("nearest_available_band")
    );
    assert_eq!(prepared.audit.quality_relaxation, None);
    assert_eq!(
        prepared.audit.snapshot_fingerprint,
        persisted.frozen.fingerprint
    );
    assert_eq!(prepared.audit.policy_version, "policy-provider-test");
    assert_eq!(prepared.audit.catalog_version, "catalog-provider-test");
    assert_eq!(prepared.audit.price_version, "price-provider-test");
}

#[derive(Clone, Debug)]
enum CapturedProviderRequest {
    Chat(ChatCompletionRequest),
    Text(CompletionRequest),
}

struct RecordingProvider {
    calls: Arc<Mutex<Vec<String>>>,
    requests: Arc<Mutex<Vec<CapturedProviderRequest>>>,
}

fn record_provider_call(
    calls: &Arc<Mutex<Vec<String>>>,
    model: &str,
    extra: &HashMap<String, serde_json::Value>,
) {
    let call = if extra.contains_key("routing_requirements") {
        format!("{model}:internal-routing-extension-leaked")
    } else {
        model.to_string()
    };
    calls.lock().unwrap().push(call);
}

#[async_trait]
impl LLMProvider for RecordingProvider {
    type Error = ProviderError;

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        self.requests
            .lock()
            .unwrap()
            .push(CapturedProviderRequest::Chat(request.clone()));
        record_provider_call(&self.calls, &request.model, &request.extra);
        if request.model == "eligible-model" {
            return serde_json::from_value(json!({
                "id": "provider-empty-response",
                "object": "chat.completion",
                "created": 0,
                "model": request.model,
                "choices": [],
                "usage": {"prompt_tokens": 1, "completion_tokens": 0, "total_tokens": 1}
            }))
            .map_err(|error| ProviderError::Internal(error.to_string()));
        }
        serde_json::from_value(json!({
            "id": "provider-test-response",
            "object": "chat.completion",
            "created": 0,
            "model": request.model,
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "ok"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        }))
        .map_err(|error| ProviderError::Internal(error.to_string()))
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, ProviderError>> + Send>>,
        ProviderError,
    > {
        self.requests
            .lock()
            .unwrap()
            .push(CapturedProviderRequest::Chat(request.clone()));
        record_provider_call(&self.calls, &request.model, &request.extra);
        let scenario = serde_json::to_value(&request)
            .ok()
            .and_then(|value| value["messages"][0]["content"].as_str().map(str::to_owned))
            .unwrap_or_default();
        let content: ChatCompletionChunk = serde_json::from_value(json!({
            "id": "provider-stream-response",
            "object": "chat.completion.chunk",
            "created": 0,
            "model": request.model,
            "choices": [{
                "index": 0,
                "delta": {"content": "ok"},
                "finish_reason": null
            }]
        }))
        .map_err(|error| ProviderError::Internal(error.to_string()))?;
        if scenario == "all-precommit-error" {
            return Ok(Box::pin(futures::stream::once(async {
                Err(ProviderError::Internal(
                    "scripted ballot exhaustion".to_string(),
                ))
            })));
        }
        if request.model == "eligible-model" {
            if scenario == "precommit-error" {
                return Ok(Box::pin(futures::stream::once(async {
                    Err(ProviderError::Internal(
                        "scripted precommit failure".to_string(),
                    ))
                })));
            }
            if scenario == "postcommit-error" {
                return Ok(Box::pin(futures::stream::iter(vec![
                    Ok(content),
                    Err(ProviderError::Internal(
                        "scripted stream failure".to_string(),
                    )),
                ])));
            }
            if scenario == "postcommit-error-observed" {
                let observed_content: ChatCompletionChunk = serde_json::from_value(json!({
                    "id": "provider-stream-response",
                    "object": "chat.completion.chunk",
                    "created": 0,
                    "model": request.model,
                    "choices": [{
                        "index": 0,
                        "delta": {"content": "ok"},
                        "finish_reason": null
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                }))
                .map_err(|error| ProviderError::Internal(error.to_string()))?;
                return Ok(Box::pin(futures::stream::iter(vec![
                    Ok(observed_content),
                    Err(ProviderError::Internal(
                        "scripted observed stream failure".to_string(),
                    )),
                ])));
            }
            if scenario == "role-only-error" {
                let role_only: ChatCompletionChunk = serde_json::from_value(json!({
                    "id": "provider-stream-response",
                    "object": "chat.completion.chunk",
                    "created": 0,
                    "model": request.model,
                    "choices": [{
                        "index": 0,
                        "delta": {"role": "assistant"},
                        "finish_reason": null
                    }]
                }))
                .map_err(|error| ProviderError::Internal(error.to_string()))?;
                return Ok(Box::pin(futures::stream::iter(vec![
                    Ok(role_only),
                    Err(ProviderError::Internal(
                        "scripted preface failure".to_string(),
                    )),
                ])));
            }
            if matches!(
                scenario.as_str(),
                "role-then-content" | "precommit-overflow" | "precommit-slow-preface"
            ) {
                let role_only: ChatCompletionChunk = serde_json::from_value(json!({
                    "id": "provider-stream-response",
                    "object": "chat.completion.chunk",
                    "created": 0,
                    "model": request.model,
                    "choices": [{
                        "index": 0,
                        "delta": {"role": "assistant"},
                        "finish_reason": null
                    }]
                }))
                .map_err(|error| ProviderError::Internal(error.to_string()))?;
                if scenario == "role-then-content" {
                    return Ok(Box::pin(
                        futures::stream::iter(vec![Ok(role_only), Ok(content)])
                            .chain(futures::stream::pending()),
                    ));
                }
                if scenario == "precommit-overflow" {
                    let chunks = (0..33)
                        .map(|_| Ok::<_, ProviderError>(role_only.clone()))
                        .collect::<Vec<_>>();
                    return Ok(Box::pin(futures::stream::iter(chunks)));
                }
                return Ok(Box::pin(futures::stream::unfold(
                    role_only,
                    |chunk| async move {
                        tokio::time::sleep(Duration::from_millis(400)).await;
                        Some((Ok::<_, ProviderError>(chunk.clone()), chunk))
                    },
                )));
            }
            if scenario == "successful-eof" {
                return Ok(Box::pin(futures::stream::once(async { Ok(content) })));
            }
            if scenario == "postcommit-timeout" {
                return Ok(Box::pin(
                    futures::stream::once(async { Ok(content) }).chain(futures::stream::pending()),
                ));
            }
            if scenario == "finish-only-pending" {
                let finish: ChatCompletionChunk = serde_json::from_value(json!({
                    "id": "provider-stream-response",
                    "object": "chat.completion.chunk",
                    "created": 0,
                    "model": request.model,
                    "choices": [{
                        "index": 0,
                        "delta": {},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                }))
                .map_err(|error| ProviderError::Internal(error.to_string()))?;
                let calls = self.calls.clone();
                return Ok(Box::pin(
                    futures::stream::once(async move {
                        calls.lock().unwrap().push("finish-only-polled".to_string());
                        Ok(finish)
                    })
                    .chain(futures::stream::pending()),
                ));
            }
            if scenario == "successful-finish" {
                let finish: ChatCompletionChunk = serde_json::from_value(json!({
                    "id": "provider-stream-response",
                    "object": "chat.completion.chunk",
                    "created": 0,
                    "model": request.model,
                    "choices": [{
                        "index": 0,
                        "delta": {},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                }))
                .map_err(|error| ProviderError::Internal(error.to_string()))?;
                return Ok(Box::pin(futures::stream::iter(vec![
                    Ok(content),
                    Ok(finish),
                ])));
            }
            return Ok(Box::pin(futures::stream::empty()));
        }
        let finish: ChatCompletionChunk = serde_json::from_value(json!({
            "id": "provider-stream-response",
            "object": "chat.completion.chunk",
            "created": 0,
            "model": request.model,
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        }))
        .map_err(|error| ProviderError::Internal(error.to_string()))?;
        Ok(Box::pin(futures::stream::iter(vec![
            Ok(content),
            Ok(finish),
        ])))
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        self.requests
            .lock()
            .unwrap()
            .push(CapturedProviderRequest::Text(request.clone()));
        record_provider_call(&self.calls, &request.model, &request.extra);
        serde_json::from_value(json!({
            "id": "provider-text-response",
            "object": "text_completion",
            "created": 0,
            "model": request.model,
            "choices": [{
                "text": "ok",
                "index": 0,
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        }))
        .map_err(|error| ProviderError::Internal(error.to_string()))
    }

    async fn completion_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<CompletionChunk, ProviderError>> + Send>>,
        ProviderError,
    > {
        self.requests
            .lock()
            .unwrap()
            .push(CapturedProviderRequest::Text(request.clone()));
        record_provider_call(&self.calls, &request.model, &request.extra);
        if request.model == "eligible-model" {
            return Ok(Box::pin(futures::stream::empty()));
        }
        let content: CompletionChunk = serde_json::from_value(json!({
            "id": "provider-text-stream-response",
            "object": "text_completion",
            "created": 0,
            "model": request.model,
            "choices": [{
                "text": "ok",
                "index": 0,
                "finish_reason": null
            }]
        }))
        .map_err(|error| ProviderError::Internal(error.to_string()))?;
        let finish: CompletionChunk = serde_json::from_value(json!({
            "id": "provider-text-stream-response",
            "object": "text_completion",
            "created": 0,
            "model": request.model,
            "choices": [{
                "text": "",
                "index": 0,
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        }))
        .map_err(|error| ProviderError::Internal(error.to_string()))?;
        Ok(Box::pin(futures::stream::iter(vec![
            Ok(content),
            Ok(finish),
        ])))
    }

    async fn embedding(
        &self,
        _request: EmbeddingRequest,
    ) -> Result<EmbeddingResponse, ProviderError> {
        Err(ProviderError::Internal("not exercised".to_string()))
    }

    async fn list_models(&self) -> Result<Vec<Model>, ProviderError> {
        Ok(Vec::new())
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "recording-provider".to_string(),
            version: "test".to_string(),
            supported_features: Vec::new(),
            rate_limits: None,
            pricing: None,
            models: Vec::new(),
        }
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_functions: false,
            supports_vision: false,
            supports_embeddings: false,
            max_context_length: None,
            max_tokens_per_request: None,
            supported_models: vec![
                "eligible-model".to_string(),
                "eligible-model-2".to_string(),
                "eligible-model-3".to_string(),
            ],
        }
    }

    async fn get_cost_info(&self, model: &str) -> Result<CostInfo, ProviderError> {
        Ok(CostInfo {
            input_cost_per_1k_tokens: 0.0,
            output_cost_per_1k_tokens: 0.0,
            currency: "USD".to_string(),
            model: model.to_string(),
        })
    }

    async fn supports_model(&self, _model: &str) -> bool {
        true
    }

    fn get_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: "http://provider.test".to_string(),
            api_key: None,
            timeout: 1,
            max_retries: 0,
            custom_headers: Default::default(),
        }
    }
}

#[derive(Debug)]
struct RecordingMoaProvider {
    calls: Arc<Mutex<u32>>,
}

#[async_trait]
impl ChatProvider for RecordingMoaProvider {
    async fn complete(
        &self,
        _model: &str,
        _prompt: &str,
        _temperature: f32,
        _max_tokens: usize,
    ) -> MoaResult<String> {
        *self.calls.lock().unwrap() += 1;
        Ok("unballoted moa response".to_string())
    }
}

async fn recording_moa_engine(calls: Arc<Mutex<u32>>) -> Arc<MoaEngine> {
    let llm = LlmAgentConfig {
        provider: MoaLlmProvider::OpenAI {
            model: "unballoted-moa-model".to_string(),
            temperature: 0.0,
            max_tokens: 16,
        },
        system_prompt: None,
        response_format: None,
        timeout_secs: 1,
        retries: None,
    };
    let mut config = MoaConfig::default();
    config.agents = vec![AgentConfig {
        name: "unballoted-moa-agent".to_string(),
        agent_type: AgentType::LLM,
        role: AgentRole::Primary,
        capabilities: Vec::new(),
        config: serde_json::to_value(llm).expect("MoA test agent serializes"),
        max_retries: 0,
        timeout_secs: 1,
    }];
    Arc::new(
        MoaEngine::from_parts(config, Arc::new(RecordingMoaProvider { calls }))
            .await
            .expect("MoA test engine starts"),
    )
}

async fn provider_boundary_state(global_attempts: u32) -> (AppState, Arc<Mutex<Vec<String>>>) {
    let (state, calls, _) = provider_boundary_state_with_requests(global_attempts).await;
    (state, calls)
}

async fn provider_boundary_state_with_requests(
    global_attempts: u32,
) -> (
    AppState,
    Arc<Mutex<Vec<String>>>,
    Arc<Mutex<Vec<CapturedProviderRequest>>>,
) {
    let mut state = build_test_state().await;
    state.routing_config = Arc::new(RoutingConfig {
        max_provider_attempts: global_attempts,
        ..RoutingConfig::default()
    });
    let calls = Arc::new(Mutex::new(Vec::new()));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(RecordingProvider {
        calls: calls.clone(),
        requests: requests.clone(),
    });
    for provider_name in ["provider-b", "provider-c", "provider-d"] {
        state
            .router
            .register_provider(provider_name, provider.clone())
            .await
            .expect("recording provider registers");
    }
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    install_budget_lifecycle(&mut state, budget);
    (state, calls, requests)
}

fn install_budget_lifecycle(
    state: &mut AppState,
    budget: Arc<dyn BudgetReservationRepositoryTrait>,
) {
    let attempts: Arc<dyn ProviderAttemptRepositoryTrait> =
        Arc::new(RecordingAttemptRepository::default());
    install_budget_lifecycle_with_attempts(state, budget, attempts);
}

fn install_budget_lifecycle_with_attempts(
    state: &mut AppState,
    budget: Arc<dyn BudgetReservationRepositoryTrait>,
    attempts: Arc<dyn ProviderAttemptRepositoryTrait>,
) {
    state.budget_reservation_repo = Some(budget.clone());
    state.provider_attempt_repo = Some(attempts.clone());
    state.attempt_lifecycle = Some(Arc::new(AttemptLifecycle::new(budget, attempts)));
}

fn classification_fixture(moa_flagged: bool) -> ClassificationExt {
    let message = Message {
        role: Role::User,
        content: Content::Text(
            "Prove the invariant and solve the constrained recurrence.".to_string(),
        ),
        name: None,
        function_call: None,
        tool_calls: None,
        tool_call_id: None,
        confidence: None,
    };

    ClassificationExt {
        complexity: MeridianComplexityEstimator::new()
            .classify(&[message], 0.7)
            .evidence,
        moa_flagged,
        required_skills: [false; 12],
        input_token_upper_bound: 64,
    }
}

fn provider_boundary_server(state: AppState, moa_flagged: bool) -> TestServer {
    TestServer::new(provider_boundary_app(
        state,
        moa_flagged,
        ProjectSettingsExt::default(),
    ))
    .expect("provider boundary server starts")
}

fn provider_boundary_app(
    state: AppState,
    moa_flagged: bool,
    project_settings: ProjectSettingsExt,
) -> Router {
    let persisted = persisted_ballot_input(2);
    let parsed: ChatCompletionRequest = serde_json::from_value(json!({
        "model": "auto",
        "messages": [{"role": "user", "content": "provider boundary test"}],
        "max_tokens": 64,
        "routing_requirements": {"compliance": ["au-residency"]}
    }))
    .expect("provider test request parses");
    let classification = classification_fixture(moa_flagged);
    Router::new()
        .route("/v1/chat/completions", post(counted_dispatch))
        .route("/v1/chat/completions/stream", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            provider_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            budget_reservation_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(Extension(ParsedRequestExt(parsed)))
        .layer(Extension(classification))
        .layer(Extension(persisted))
        .layer(Extension(project_settings))
        .layer(middleware::from_fn(request_logging))
        .with_state(state)
}

fn budgeted_provider_boundary_server(state: AppState) -> TestServer {
    let persisted = persisted_ballot_input(2);
    let parsed: ChatCompletionRequest = serde_json::from_value(json!({
        "model": "auto",
        "messages": [{"role": "user", "content": "budget boundary test"}],
        "max_tokens": 64,
        "routing_requirements": {"compliance": ["au-residency"]}
    }))
    .expect("budget test request parses");
    let classification = classification_fixture(false);
    let app = Router::new()
        .route("/v1/chat/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            provider_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            budget_reservation_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(Extension(ParsedRequestExt(parsed)))
        .layer(Extension(classification))
        .layer(Extension(persisted))
        .layer(middleware::from_fn(request_logging))
        .with_state(state);
    TestServer::new(app).expect("budget boundary server starts")
}

fn text_provider_boundary_server(state: AppState) -> TestServer {
    let persisted = persisted_ballot_input(2);
    let parsed = GenerationRequest::from_http(
        "/v1/completions",
        br#"{
            "model":"auto",
            "prompt":"buffered text boundary test",
            "max_tokens":64,
            "routing_requirements":{"compliance":["au-residency"]}
        }"#,
    )
    .expect("text boundary request parses");
    let classification = classification_fixture(false);
    let app = Router::new()
        .route("/v1/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            provider_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            budget_reservation_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(Extension(ParsedGenerationExt(parsed)))
        .layer(Extension(classification))
        .layer(Extension(persisted))
        .layer(middleware::from_fn(request_logging))
        .with_state(state);
    TestServer::new(app).expect("text provider boundary server starts")
}

fn streaming_chat_provider_boundary_server(state: AppState, prompt: &str) -> TestServer {
    TestServer::new(streaming_chat_provider_boundary_app(state, prompt))
        .expect("streaming chat provider boundary server starts")
}

fn streaming_chat_provider_boundary_app(state: AppState, prompt: &str) -> Router {
    let persisted = persisted_ballot_input(2);
    let wire = serde_json::to_vec(&json!({
        "model": "auto",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 64,
        "stream": true,
        "routing_requirements": {"compliance": ["au-residency"]}
    }))
    .expect("streaming chat request serializes");
    let parsed = GenerationRequest::from_http("/v1/chat/completions", &wire)
        .expect("streaming chat request parses");
    let classification = classification_fixture(false);
    Router::new()
        .route("/v1/chat/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            provider_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            budget_reservation_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(Extension(ParsedGenerationExt(parsed)))
        .layer(Extension(classification))
        .layer(Extension(persisted))
        .layer(middleware::from_fn(request_logging))
        .with_state(state)
}

async fn raw_streaming_chat_response(state: AppState, prompt: &str) -> axum::response::Response {
    let mut app = streaming_chat_provider_boundary_app(state, prompt);
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "model": "auto",
                "messages": [{"role": "user", "content": prompt}],
                "stream": true
            }))
            .expect("raw stream request serializes"),
        ))
        .expect("raw stream request builds");
    tower::Service::call(&mut app, request)
        .await
        .expect("raw stream request succeeds")
}

fn streaming_text_provider_boundary_server(state: AppState) -> TestServer {
    let persisted = persisted_ballot_input(2);
    let parsed = GenerationRequest::from_http(
        "/v1/completions",
        br#"{
            "model":"auto",
            "prompt":"text stream boundary test",
            "max_tokens":64,
            "stream":true,
            "routing_requirements":{"compliance":["au-residency"]}
        }"#,
    )
    .expect("streaming text request parses");
    let classification = classification_fixture(false);
    let app = Router::new()
        .route("/v1/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            provider_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            budget_reservation_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(Extension(ParsedGenerationExt(parsed)))
        .layer(Extension(classification))
        .layer(Extension(persisted))
        .layer(middleware::from_fn(request_logging))
        .with_state(state);
    TestServer::new(app).expect("streaming text provider boundary server starts")
}

async fn r2_output_budget_provider_server(
    output_budgets: [u32; 2],
) -> (
    TestServer,
    Arc<Mutex<Vec<CapturedProviderRequest>>>,
    Arc<RecordingAttemptRepository>,
    Arc<Mutex<Vec<ReserveBudgetCommand>>>,
) {
    let (mut state, _, requests) = provider_boundary_state_with_requests(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let reservations = Arc::new(Mutex::new(Vec::new()));
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: reservations.clone(),
        committed_attempts: Some(attempts.clone()),
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    state.routing_snapshot_preparer = Some(Arc::new(R2BallotSnapshotPreparer {
        catalog: CatalogSnapshot::new(vec![
            catalog_model("eligible-model", "provider-b", true),
            catalog_model("eligible-model-2", "provider-c", true),
        ]),
        output_budgets,
    }));
    let identity = ResolvedProjectIdentity {
        project_id: "project-a".to_string(),
        org_id: "org-a".to_string(),
        principal_id: "user-a".to_string(),
        auth_kind: AuthKind::Jwt,
    };
    let app = Router::new()
        .route("/v1/chat/completions", post(counted_dispatch))
        .route("/v1/chat/completions/stream", post(counted_dispatch))
        .route("/v1/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            provider_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            budget_reservation_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routing_snapshot_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            classification_middleware_with_state,
        ))
        .layer(Extension(identity))
        .layer(middleware::from_fn(request_logging))
        .with_state(state);
    (
        TestServer::new(app).expect("R2 output-budget server starts"),
        requests,
        attempts,
        reservations,
    )
}

fn assert_p1_routing_audit_headers(response: &axum_test::TestResponse) {
    let persisted = persisted_ballot_input(2);
    let expected_ballot_id = prepare_selection(&persisted)
        .expect("test routing input selects")
        .ballot
        .content_id()
        .expect("test ballot has canonical identity");
    assert_eq!(response.header("x-gaussmeridian-desired-band"), "frontier");
    assert_eq!(response.header("x-gaussmeridian-selected-band"), "advanced");
    assert_eq!(
        response.header("x-gaussmeridian-band-reason"),
        "nearest_available_band"
    );
    assert_eq!(
        response.header("x-gaussmeridian-quality-relaxation"),
        "none"
    );
    assert_eq!(
        response.header("x-gaussmeridian-policy-version"),
        "policy-provider-test"
    );
    assert_eq!(
        response.header("x-gaussmeridian-catalog-version"),
        "catalog-provider-test"
    );
    assert_eq!(
        response.header("x-gaussmeridian-price-version"),
        "price-provider-test"
    );
    assert_eq!(response.header("x-gaussmeridian-r2-status"), "predecessor");
    assert_eq!(response.header("x-gaussmeridian-output-budget"), "64");
    assert_eq!(
        response.header("x-gaussmeridian-output-budget-version"),
        "none"
    );
    assert_eq!(
        response.header("x-gaussmeridian-predictor-version"),
        "carrot-knn/v1"
    );
    assert_eq!(
        response.header("x-gaussmeridian-predictor-feature-version"),
        "carrot-runtime-features/v1"
    );
    assert_eq!(response.header("x-gaussmeridian-learner-state-id"), "none");
    assert_eq!(
        response.header("x-gaussmeridian-predictor-status"),
        "unavailable"
    );
    assert_eq!(
        response.header("x-gaussmeridian-predictor-outcome-estimates"),
        "0"
    );
    assert_eq!(
        response.header("x-gaussmeridian-predictor-cost-estimates"),
        "0"
    );
    assert_eq!(response.header("x-gaussmeridian-bella-status"), "inactive");
    assert_eq!(response.header("x-gaussmeridian-bella-state-id"), "none");
    assert_eq!(
        response.header("x-gaussmeridian-bella-taxonomy-version"),
        "none"
    );
    assert_eq!(
        response.header("x-gaussmeridian-bella-profiler-version"),
        "none"
    );
    assert_eq!(
        response.header("x-gaussmeridian-bella-critic-version"),
        "none"
    );
    assert_eq!(
        response.header("x-gaussmeridian-bella-fallback-reason"),
        "no_active_state"
    );
    assert_eq!(
        response.header("x-gaussmeridian-bella-task-skill-count"),
        "0"
    );
    assert_eq!(
        response.header("x-gaussmeridian-bella-capable-route-count"),
        "0"
    );
    assert_eq!(
        response.header("x-gaussmeridian-predictor-abstained-routes"),
        "0"
    );
    assert_eq!(
        response.header("x-gaussmeridian-predictor-fallback-reason"),
        "no_active_state"
    );
    assert_eq!(
        response.header("x-gaussmeridian-ballot-id"),
        expected_ballot_id
    );
    assert_eq!(
        response.header("x-gaussmeridian-snapshot-fingerprint"),
        persisted.frozen.fingerprint
    );
}

#[tokio::test]
async fn applied_r2_budget_reaches_provider_across_all_four_transports_and_fallbacks() {
    let (server, requests, attempts, reservations) =
        r2_output_budget_provider_server([R2_PRIMARY_OUTPUT_BUDGET, R2_PRIMARY_OUTPUT_BUDGET])
            .await;
    let cases = [
        (
            "/v1/chat/completions",
            json!({
                "model": "auto",
                "messages": [
                    {"role": "system", "content": "caller policy"},
                    {"role": "user", "content": "solve this"}
                ],
                "max_tokens": R2_CALLER_OUTPUT_CEILING
            }),
            false,
            true,
            2,
        ),
        (
            "/v1/chat/completions/stream",
            json!({
                "model": "auto",
                "messages": [
                    {"role": "system", "content": "caller policy"},
                    {"role": "user", "content": "solve this"}
                ],
                "max_tokens": R2_CALLER_OUTPUT_CEILING
            }),
            true,
            true,
            2,
        ),
        (
            "/v1/completions",
            json!({
                "model": "auto",
                "prompt": "solve this",
                "max_tokens": R2_CALLER_OUTPUT_CEILING
            }),
            false,
            false,
            1,
        ),
        (
            "/v1/completions",
            json!({
                "model": "auto",
                "prompt": "solve this",
                "max_tokens": R2_CALLER_OUTPUT_CEILING,
                "stream": true
            }),
            true,
            false,
            2,
        ),
    ];

    for (index, (path, body, streaming, chat, expected_attempts)) in cases.into_iter().enumerate() {
        let capture_start = requests.lock().unwrap().len();
        let attempt_start = attempts.records.lock().unwrap().len();
        let reservation_start = reservations.lock().unwrap().len();
        let response = server
            .post(path)
            .add_header("idempotency-key", format!("p4-output-budget-{index}"))
            .json(&body)
            .await;

        assert_eq!(response.status_code(), StatusCode::OK, "case {index}");
        assert_eq!(
            response.header("x-gaussmeridian-r2-status"),
            "applied",
            "case {index}"
        );
        assert_eq!(
            response.header("x-gaussmeridian-output-budget-version"),
            OUTPUT_BUDGET_INSTRUCTION_VERSION,
            "case {index}"
        );
        assert_eq!(
            response.header("x-gaussmeridian-output-budget"),
            R2_PRIMARY_OUTPUT_BUDGET.to_string(),
            "case {index}"
        );
        assert_ne!(
            R2_PRIMARY_OUTPUT_BUDGET, R2_CALLER_OUTPUT_CEILING,
            "the test must distinguish the selected action from the caller ceiling"
        );
        let instruction = output_budget_constraint_fixture(R2_PRIMARY_OUTPUT_BUDGET).instruction;
        let _ = response.text();

        let reservation_commands = reservations.lock().unwrap()[reservation_start..].to_vec();
        assert_eq!(reservation_commands.len(), 1, "case {index}");
        let expected_reservation = expected_r2_reservation_amount(
            &body,
            [R2_PRIMARY_OUTPUT_BUDGET, R2_PRIMARY_OUTPUT_BUDGET],
        );
        assert!(
            (reservation_commands[0].amount - expected_reservation).abs() < 1e-12,
            "case {index} reservation did not include both action-specific constraint bounds"
        );

        let captured = requests.lock().unwrap()[capture_start..].to_vec();
        assert_eq!(captured.len(), expected_attempts, "case {index}");
        for request in captured {
            match request {
                CapturedProviderRequest::Chat(request) => {
                    assert!(chat, "case {index} used the wrong endpoint");
                    assert_eq!(
                        request.max_tokens,
                        Some(R2_PRIMARY_OUTPUT_BUDGET),
                        "case {index}"
                    );
                    assert_eq!(request.stream, Some(streaming), "case {index}");
                    assert!(!request.extra.contains_key("routing_requirements"));
                    let instruction_positions = request
                        .messages
                        .iter()
                        .enumerate()
                        .filter_map(|(position, message)| match &message.content {
                            Content::Text(content) if content == &instruction => Some(position),
                            Content::Text(_) | Content::Parts(_) => None,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(instruction_positions, [1], "case {index}");
                }
                CapturedProviderRequest::Text(request) => {
                    assert!(!chat, "case {index} used the wrong endpoint");
                    assert_eq!(
                        request.max_tokens,
                        Some(R2_PRIMARY_OUTPUT_BUDGET),
                        "case {index}"
                    );
                    assert_eq!(request.stream, Some(streaming), "case {index}");
                    assert!(!request.extra.contains_key("routing_requirements"));
                    assert_eq!(
                        request.prompt,
                        format!("{instruction}\n\nsolve this"),
                        "case {index}"
                    );
                }
            }
        }

        let records = attempts.records.lock().unwrap()[attempt_start..].to_vec();
        assert_eq!(records.len(), expected_attempts, "case {index}");
        for record in &records {
            assert_eq!(
                record.requested_output_tokens, R2_PRIMARY_OUTPUT_BUDGET,
                "case {index}"
            );
            assert_eq!(
                record.output_constraint_version.as_deref(),
                Some(OUTPUT_BUDGET_INSTRUCTION_VERSION),
                "case {index}"
            );
            assert_eq!(
                record.actual_output_tokens.is_some(),
                record.token_limit_compliant.is_some(),
                "case {index} must not invent partial output evidence"
            );
        }
        let successful = records.last().expect("successful attempt is retained");
        assert_eq!(successful.actual_output_tokens, Some(1), "case {index}");
        assert_eq!(
            successful.finish_reason.as_deref(),
            Some("stop"),
            "case {index}"
        );
        assert_eq!(successful.token_limit_compliant, Some(true), "case {index}");
    }
}

#[tokio::test]
async fn each_fallback_attempt_uses_its_own_output_budget_contract() {
    let (server, requests, attempts, reservations) =
        r2_output_budget_provider_server([R2_PRIMARY_OUTPUT_BUDGET, R2_FALLBACK_OUTPUT_BUDGET])
            .await;
    let cases = [
        (
            "/v1/chat/completions",
            json!({
                "model": "auto",
                "messages": [
                    {"role": "system", "content": "caller policy"},
                    {"role": "user", "content": "solve this"}
                ],
                "max_tokens": R2_CALLER_OUTPUT_CEILING
            }),
            true,
        ),
        (
            "/v1/chat/completions/stream",
            json!({
                "model": "auto",
                "messages": [
                    {"role": "system", "content": "caller policy"},
                    {"role": "user", "content": "solve this"}
                ],
                "max_tokens": R2_CALLER_OUTPUT_CEILING
            }),
            true,
        ),
        (
            "/v1/completions",
            json!({
                "model": "auto",
                "prompt": "solve this",
                "max_tokens": R2_CALLER_OUTPUT_CEILING,
                "stream": true
            }),
            false,
        ),
    ];

    for (index, (path, body, chat)) in cases.into_iter().enumerate() {
        let capture_start = requests.lock().unwrap().len();
        let attempt_start = attempts.records.lock().unwrap().len();
        let reservation_start = reservations.lock().unwrap().len();
        let response = server
            .post(path)
            .add_header("idempotency-key", format!("p4-distinct-budget-{index}"))
            .json(&body)
            .await;

        assert_eq!(response.status_code(), StatusCode::OK, "case {index}");
        assert_eq!(
            response.header("x-gaussmeridian-output-budget"),
            R2_FALLBACK_OUTPUT_BUDGET.to_string(),
            "the successful fallback action owns the response budget"
        );
        let _ = response.text();

        let reservation_commands = reservations.lock().unwrap()[reservation_start..].to_vec();
        assert_eq!(reservation_commands.len(), 1, "case {index}");
        let expected_reservation = expected_r2_reservation_amount(
            &body,
            [R2_PRIMARY_OUTPUT_BUDGET, R2_FALLBACK_OUTPUT_BUDGET],
        );
        assert!(
            (reservation_commands[0].amount - expected_reservation).abs() < 1e-12,
            "case {index} reservation did not include each distinct action constraint bound"
        );

        let captured = requests.lock().unwrap()[capture_start..].to_vec();
        assert_eq!(captured.len(), 2, "case {index}");
        for (attempt_index, (request, expected_budget)) in captured
            .into_iter()
            .zip([R2_PRIMARY_OUTPUT_BUDGET, R2_FALLBACK_OUTPUT_BUDGET])
            .enumerate()
        {
            let instruction = output_budget_constraint_fixture(expected_budget).instruction;
            match request {
                CapturedProviderRequest::Chat(request) => {
                    assert!(chat, "case {index} attempt {attempt_index}");
                    assert_eq!(request.max_tokens, Some(expected_budget));
                    assert_eq!(
                        request
                            .messages
                            .iter()
                            .filter(|message| match &message.content {
                                Content::Text(content) => content == &instruction,
                                Content::Parts(_) => false,
                            })
                            .count(),
                        1,
                        "case {index} attempt {attempt_index}"
                    );
                }
                CapturedProviderRequest::Text(request) => {
                    assert!(!chat, "case {index} attempt {attempt_index}");
                    assert_eq!(request.max_tokens, Some(expected_budget));
                    assert_eq!(
                        request.prompt,
                        format!("{instruction}\n\nsolve this"),
                        "case {index} attempt {attempt_index}"
                    );
                }
            }
        }

        let records = attempts.records.lock().unwrap()[attempt_start..].to_vec();
        assert_eq!(records.len(), 2, "case {index}");
        assert_eq!(
            records
                .iter()
                .map(|record| record.requested_output_tokens)
                .collect::<Vec<_>>(),
            [R2_PRIMARY_OUTPUT_BUDGET, R2_FALLBACK_OUTPUT_BUDGET],
            "case {index}"
        );
        assert!(records.iter().all(|record| {
            record.output_constraint_version.as_deref() == Some(OUTPUT_BUDGET_INSTRUCTION_VERSION)
        }));
        assert_eq!(records[1].actual_output_tokens, Some(1), "case {index}");
        assert_eq!(
            records[1].finish_reason.as_deref(),
            Some("stop"),
            "case {index}"
        );
        assert_eq!(records[1].token_limit_compliant, Some(true), "case {index}");
    }
}

#[tokio::test]
async fn invalid_output_budget_contract_is_rejected_before_every_transport_reservation() {
    let cases = [
        (
            "/v1/chat/completions",
            json!({
                "model": "auto",
                "messages": [{"role": "user", "content": "solve this"}],
                "max_tokens": R2_CALLER_OUTPUT_CEILING
            }),
        ),
        (
            "/v1/chat/completions/stream",
            json!({
                "model": "auto",
                "messages": [{"role": "user", "content": "solve this"}],
                "max_tokens": R2_CALLER_OUTPUT_CEILING,
                "stream": true
            }),
        ),
        (
            "/v1/completions",
            json!({
                "model": "auto",
                "prompt": "solve this",
                "max_tokens": R2_CALLER_OUTPUT_CEILING
            }),
        ),
        (
            "/v1/completions",
            json!({
                "model": "auto",
                "prompt": "solve this",
                "max_tokens": R2_CALLER_OUTPUT_CEILING,
                "stream": true
            }),
        ),
    ];

    for (contract_index, invalid_version) in [true, false].into_iter().enumerate() {
        for (transport_index, (path, body)) in cases.iter().enumerate() {
            let wire = serde_json::to_vec(body).expect("request serializes");
            let generation =
                GenerationRequest::from_http(path, &wire).expect("generation request parses");
            let persisted = persisted_ballot_input(2);
            let prepared = prepare_selection(&persisted).expect("selection prepares");
            let mut audit = prepared.audit;
            let mut candidates = prepared.candidates;
            if invalid_version {
                audit.r2.instruction_version = Some("unsupported-output-budget/v9".to_string());
            } else {
                candidates[0].output_token_budget = 0;
            }
            let commands = Arc::new(Mutex::new(Vec::new()));
            let budget = Arc::new(RecordingBudgetRepository {
                behavior: ReservationBehavior::Reserve,
                commands: commands.clone(),
                committed_attempts: None,
            });
            let mut state = build_test_state().await;
            install_budget_lifecycle(&mut state, budget);
            let app = Router::new()
                .route(path, post(authorized_generation_dispatch))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    budget_reservation_middleware_with_state,
                ))
                .layer(Extension(ExecutionRequestIdentity {
                    request_id: format!(
                        "invalid-contract-request-{contract_index}-{transport_index}"
                    ),
                    idempotency_key: format!(
                        "invalid-contract-key-{contract_index}-{transport_index}"
                    ),
                }))
                .layer(Extension(audit))
                .layer(Extension(SelectionExt { candidates }))
                .layer(Extension(prepared.quote))
                .layer(Extension(prepared.ballot))
                .layer(Extension(ParsedGenerationExt(generation)))
                .layer(Extension(persisted))
                .with_state(state);
            let server = TestServer::new(app).expect("invalid-contract server starts");

            let response = server.post(path).json(body).await;

            assert_eq!(
                response.status_code(),
                StatusCode::SERVICE_UNAVAILABLE,
                "contract {contract_index}, transport {transport_index}"
            );
            assert!(
                commands.lock().unwrap().is_empty(),
                "contract {contract_index}, transport {transport_index} created a reservation"
            );
        }
    }
}

#[async_trait]
impl RoutingIdentityResolver for RequestedProjectResolver {
    async fn resolve_identity(
        &self,
        input: ResolveProjectInput<'_>,
    ) -> Result<ResolvedProjectIdentity, IdentityError> {
        let ResolveProjectInput::Jwt {
            user_id,
            requested_project_id: Some(project_id),
        } = input
        else {
            return Err(IdentityError::ProjectContextRequired);
        };
        Ok(ResolvedProjectIdentity {
            project_id: project_id.to_string(),
            org_id: format!("org-{project_id}"),
            principal_id: user_id.to_string(),
            auth_kind: AuthKind::Jwt,
        })
    }
}

#[async_trait]
impl RoutingIdentityResolver for StubIdentityResolver {
    async fn resolve_identity(
        &self,
        _input: ResolveProjectInput<'_>,
    ) -> Result<ResolvedProjectIdentity, IdentityError> {
        self.result.clone()
    }
}

struct InfallibleMemCache<K, V> {
    inner: MemoryCache<K, V>,
}

#[async_trait]
impl<K, V> Cache<K, V> for InfallibleMemCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    type Error = Infallible;

    async fn get(&self, key: &K) -> Result<Option<V>, Self::Error> {
        self.inner.get(key).await.map_err(|_| unreachable!())
    }

    async fn set(&self, key: K, value: V, ttl: Option<Duration>) -> Result<(), Self::Error> {
        self.inner
            .set(key, value, ttl)
            .await
            .map_err(|_| unreachable!())
    }

    async fn delete(&self, key: &K) -> Result<(), Self::Error> {
        self.inner.delete(key).await.map_err(|_| unreachable!())
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        self.inner.clear().await.map_err(|_| unreachable!())
    }

    async fn exists(&self, key: &K) -> Result<bool, Self::Error> {
        self.inner.exists(key).await.map_err(|_| unreachable!())
    }

    async fn size(&self) -> Result<usize, Self::Error> {
        self.inner.size().await.map_err(|_| unreachable!())
    }

    async fn get_stats(&self) -> Result<gaussmeridian_cache::stats::CacheStats, Self::Error> {
        Ok(self.inner.get_stats())
    }
}

async fn build_test_state() -> AppState {
    let config = Arc::new(gaussmeridian_config::AppConfig::default());
    let router_cache: Arc<dyn Cache<CacheKey, CacheValue, Error = Infallible>> =
        Arc::new(InfallibleMemCache {
            inner: MemoryCache::new(1000, Duration::from_secs(3600)),
        });
    let router = Arc::new(GaussMeridian::new(
        router_cache,
        None,
        Arc::new(LeastConnectionsLoadBalancer),
        None,
    ));
    let auth_manager = Arc::new(gaussmeridian_auth::AuthManager::new(
        JwtManager::new(JWT_SECRET),
        ApiKeyManager::new(),
        RBACManager::new(),
    ));

    AppState::new(
        router,
        config,
        None,
        auth_manager,
        Arc::new(RateLimiter::with_default_config(RateLimitConfig {
            requests_per_minute: 100,
            tokens_per_minute: 100_000,
            window_size: Duration::from_secs(60),
        })),
        false,
        None,
        None,
        Arc::new(MokaL1Cache::new(1000, Duration::from_secs(3600))),
        None,
        None,
        None,
        Arc::new(RoutingConfig::default()),
        Arc::new(vec![]),
        Arc::new(reqwest::Client::new()),
        Arc::new(Mutex::new(RoutingMetricsData::default())),
        None,
        Arc::new(gaussmeridian_core::GuardrailConfig::default()),
        Arc::new(gaussmeridian_core::CascadeConfig::default()),
    )
}

fn jwt_for(user_id: &str) -> String {
    let claims = std::collections::HashMap::from([(
        "sub".to_string(),
        serde_json::Value::String(user_id.to_string()),
    )]);
    JwtManager::new(JWT_SECRET)
        .create_token(&claims)
        .expect("test JWT mints")
}

fn test_server(state: AppState) -> TestServer {
    let app = Router::new()
        .route("/v1/chat/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware_with_state,
        ))
        .with_state(state);
    TestServer::new(app).expect("test server starts")
}

async fn counted_dispatch(State(state): State<AppState>) -> StatusCode {
    state.routing_metrics.lock().unwrap().total_requests += 1;
    StatusCode::OK
}

async fn cache_probe_dispatch(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.routing_metrics.lock().unwrap().total_requests += 1;
    Json(json!({
        "id": "stream-cache-probe",
        "object": "chat.completion.chunk",
        "model": "eligible-model",
    }))
}

async fn snapshot_dispatch(
    State(state): State<AppState>,
    Extension(persisted): Extension<PersistedRoutingInput>,
) -> Json<serde_json::Value> {
    state.routing_metrics.lock().unwrap().total_requests += 1;
    Json(json!({
        "project_id": persisted.identity.project_id,
        "snapshot_project_id": persisted.snapshot.project_id,
        "fingerprint": persisted.frozen.fingerprint,
    }))
}

async fn ballot_dispatch(
    State(state): State<AppState>,
    Extension(ballot): Extension<RoutingBallot>,
    Extension(quote): Extension<TrajectoryReservationQuote>,
    Extension(selection): Extension<SelectionExt>,
) -> Json<serde_json::Value> {
    state.routing_metrics.lock().unwrap().total_requests += 1;
    Json(json!({
        "models": ballot
            .entries()
            .iter()
            .map(|entry| entry.model_id.as_str())
            .collect::<Vec<_>>(),
        "dispatch_models": selection
            .candidates
            .iter()
            .map(|candidate| candidate.model_name.as_str())
            .collect::<Vec<_>>(),
        "reserved_attempts": quote.provider_attempts,
    }))
}

async fn authorized_generation_dispatch(
    Extension(generation): Extension<ParsedGenerationExt>,
    Extension(authorization): Extension<AuthorizedBudgetReservation>,
    Extension(ballot): Extension<RoutingBallot>,
) -> Json<serde_json::Value> {
    Json(json!({
        "endpoint": match generation.0.endpoint() {
            GenerationEndpoint::Chat => "chat",
            GenerationEndpoint::Text => "text",
        },
        "transport": match generation.0.transport() {
            GenerationTransport::Buffered => "buffered",
            GenerationTransport::Streaming => "streaming",
        },
        "reservation_id": authorization.record.reservation_id,
        "ballot_entries": ballot.entries().len(),
    }))
}

async fn learned_generation_dispatch(
    Extension(generation): Extension<ParsedGenerationExt>,
    Extension(_authorization): Extension<AuthorizedBudgetReservation>,
    Extension(persisted): Extension<PersistedRoutingInput>,
    Extension(ballot): Extension<RoutingBallot>,
    Extension(selection): Extension<SelectionExt>,
    Extension(audit): Extension<SelectionAuditProjection>,
) -> axum::response::Response {
    let learner_state_id = match &persisted.snapshot.evidence.predictions {
        FrozenPredictionEvidence::Active(predictions) => {
            predictions.provenance().learner_state_id.as_str()
        }
        FrozenPredictionEvidence::Unavailable { .. } => "unavailable",
    };
    let learned_predictions_used = ballot.entries().first().is_some_and(|entry| {
        matches!(
            (&entry.outcome_prediction, &entry.cost_prediction),
            (
                PredictionEstimate::Estimated { .. },
                PredictionEstimate::Estimated { .. }
            )
        )
    });
    let ballot_prefix = ballot
        .entries()
        .iter()
        .take(selection.candidates.len())
        .map(|entry| (&entry.provider_id, &entry.model_id))
        .collect::<Vec<_>>();
    let selected_routes = selection
        .candidates
        .iter()
        .map(|candidate| (&candidate.provider_name, &candidate.model_name))
        .collect::<Vec<_>>();
    let ballot_id = ballot.content_id().expect("learned ballot has an identity");
    let response = json!({
        "endpoint": match generation.0.endpoint() {
            GenerationEndpoint::Chat => "chat",
            GenerationEndpoint::Text => "text",
        },
        "transport": match generation.0.transport() {
            GenerationTransport::Buffered => "buffered",
            GenerationTransport::Streaming => "streaming",
        },
        "ballot_id": ballot_id,
        "snapshot_fingerprint": persisted.frozen.fingerprint,
        "routing_context": persisted.snapshot.request,
        "models": ballot
            .entries()
            .iter()
            .map(|entry| entry.model_id.as_str())
            .collect::<Vec<_>>(),
        "entry_economics": ballot
            .entries()
            .iter()
            .map(|entry| json!({
                "model": entry.model_id,
                "output_token_budget": entry.output_token_budget,
                "expected_provider_cost": entry.expected_provider_cost,
                "risk": entry.risk,
            }))
            .collect::<Vec<_>>(),
        "learner_state_id": learner_state_id,
        "hard_exclusions": ballot
            .exclusions
            .iter()
            .map(|exclusion| exclusion.model_id.as_str())
            .collect::<Vec<_>>(),
        "learned_predictions_used": learned_predictions_used,
        "selection_matches_ballot": selected_routes == ballot_prefix,
    });
    with_routing_audit_headers(
        axum::http::Response::builder(),
        &audit,
        response["ballot_id"]
            .as_str()
            .expect("ballot ID remains a string"),
        selection
            .candidates
            .first()
            .expect("learned selection is non-empty")
            .output_token_budget,
    )
    .status(StatusCode::OK)
    .header("content-type", "application/json")
    .body(Body::from(
        serde_json::to_vec(&response).expect("learned response serializes"),
    ))
    .expect("learned response builds")
}

async fn identity_dispatch(
    State(state): State<AppState>,
    Extension(identity): Extension<ResolvedProjectIdentity>,
    Extension(settings): Extension<ProjectSettingsExt>,
) -> Json<serde_json::Value> {
    state.routing_metrics.lock().unwrap().total_requests += 1;
    Json(json!({
        "model": "identity-probe",
        "project_id": identity.project_id,
        "settings_project_id": settings.project_id,
    }))
}

#[tokio::test]
async fn generation_settings_use_the_same_canonical_project_identity() {
    let mut state = build_test_state().await;
    state.routing_identity_resolver = Some(Arc::new(RequestedProjectResolver));
    let app = Router::new()
        .route("/v1/chat/completions", post(identity_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware_with_state,
        ))
        .with_state(state.clone());
    let server = TestServer::new(app).expect("test server starts");

    let response = server
        .post("/v1/chat/completions")
        .add_header("authorization", format!("Bearer {}", jwt_for("user-a")))
        .add_header("x-project-id", "project-a")
        .json(&json!({"model": "auto", "messages": [{"role": "user", "content": "Hi"}]}))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["project_id"], "project-a");
    assert_eq!(body["settings_project_id"], "project-a");
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 1);
}

#[tokio::test]
async fn authoritative_snapshot_failure_is_sanitized_before_dispatch() {
    let mut state = build_test_state().await;
    state.routing_identity_resolver = Some(Arc::new(RequestedProjectResolver));
    state.routing_snapshot_preparer = Some(Arc::new(UnavailableSnapshotPreparer));
    let app = Router::new()
        .route("/v1/chat/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routing_snapshot_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            classification_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware_with_state,
        ))
        .with_state(state.clone());
    let server = TestServer::new(app).expect("test server starts");

    let response = server
        .post("/v1/chat/completions")
        .add_header("authorization", format!("Bearer {}", jwt_for("user-a")))
        .add_header("x-project-id", "project-a")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "Prove the spectral theorem"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["code"], "service_unavailable");
    assert_eq!(
        body["error"]["message"],
        "Service is temporarily unavailable"
    );
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 0);
}

#[tokio::test]
async fn malformed_explicit_requirements_return_http_400_before_snapshot_work() {
    let mut state = build_test_state().await;
    let prepared_projects = Arc::new(Mutex::new(Vec::new()));
    state.routing_snapshot_preparer = Some(Arc::new(SuccessfulSnapshotPreparer {
        prepared_projects: prepared_projects.clone(),
    }));
    let identity = ResolvedProjectIdentity {
        project_id: "project-a".to_string(),
        org_id: "org-a".to_string(),
        principal_id: "user-a".to_string(),
        auth_kind: AuthKind::Jwt,
    };
    let app = Router::new()
        .route("/v1/chat/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routing_snapshot_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            classification_middleware_with_state,
        ))
        .layer(Extension(identity))
        .with_state(state.clone());
    let server = TestServer::new(app).expect("requirement validation server starts");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "hello"}],
            "routing_requirements": {
                "absolute_capability_ceiling": "unbounded"
            }
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["code"], "invalid_field_format");
    assert_eq!(body["error"]["param"], "routing_requirements");
    assert!(prepared_projects.lock().unwrap().is_empty());
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 0);
}

#[tokio::test]
async fn persisted_snapshot_is_attached_before_dispatch() {
    let mut state = build_test_state().await;
    state.routing_identity_resolver = Some(Arc::new(RequestedProjectResolver));
    let prepared_projects = Arc::new(Mutex::new(Vec::new()));
    state.routing_snapshot_preparer = Some(Arc::new(SuccessfulSnapshotPreparer {
        prepared_projects: prepared_projects.clone(),
    }));
    let app = Router::new()
        .route("/v1/chat/completions", post(snapshot_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routing_snapshot_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            classification_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware_with_state,
        ))
        .with_state(state.clone());
    let server = TestServer::new(app).expect("test server starts");

    let response = server
        .post("/v1/chat/completions")
        .add_header("authorization", format!("Bearer {}", jwt_for("user-a")))
        .add_header("x-project-id", "project-a")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "Prove the spectral theorem"}],
            "max_tokens": 2048
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["project_id"], "project-a");
    assert_eq!(body["snapshot_project_id"], "project-a");
    assert_eq!(body["fingerprint"].as_str().unwrap().len(), 64);
    assert_eq!(prepared_projects.lock().unwrap().as_slice(), ["project-a"]);
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 1);
}

#[tokio::test]
async fn selection_requires_the_authoritative_persisted_snapshot() {
    let state = build_test_state().await;
    let app = Router::new()
        .route("/v1/chat/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            classification_middleware_with_state,
        ))
        .with_state(state.clone());
    let server = TestServer::new(app).expect("test server starts");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "Prove the spectral theorem"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 0);
}

#[tokio::test]
async fn empty_hard_eligible_set_returns_typed_sanitized_http_503() {
    let state = build_test_state().await;
    let mut persisted = persisted_ballot_input(2);
    persisted.snapshot.catalog =
        CatalogSnapshot::new(vec![catalog_model("blocked-model", "provider-a", false)]);
    persisted.frozen = persisted
        .snapshot
        .freeze()
        .expect("hard-ineligible snapshot freezes");
    let app = Router::new()
        .route("/v1/chat/completions", post(counted_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(Extension(classification_fixture(false)))
        .layer(Extension(persisted))
        .with_state(state.clone());
    let server = TestServer::new(app).expect("hard eligibility server starts");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["type"], "routing_unavailable_error");
    assert_eq!(body["error"]["code"], "no_hard_eligible_models");
    assert_eq!(
        body["error"]["message"],
        "No model satisfies every hard routing constraint"
    );
    assert_eq!(
        body["error"]["exclusions"],
        json!([{
            "model_id": "blocked-model",
            "provider_id": "provider-a",
            "reasons": [{"code": "compliance_denied"}]
        }])
    );
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 0);
}

#[tokio::test]
async fn selection_ballot_never_resurrects_a_hard_ineligible_model() {
    let mut state = build_test_state().await;
    state.routing_snapshot_preparer = Some(Arc::new(BallotSnapshotPreparer {
        catalog: CatalogSnapshot::new(vec![
            catalog_model("forbidden-model", "provider-a", false),
            catalog_model("eligible-model", "provider-b", true),
            catalog_model("eligible-model-2", "provider-c", true),
            catalog_model("eligible-model-3", "provider-d", true),
        ]),
    }));
    let identity = ResolvedProjectIdentity {
        project_id: "project-a".to_string(),
        org_id: "org-a".to_string(),
        principal_id: "user-a".to_string(),
        auth_kind: AuthKind::Jwt,
    };
    let app = Router::new()
        .route("/v1/chat/completions", post(ballot_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routing_snapshot_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            classification_middleware_with_state,
        ))
        .layer(Extension(identity))
        .with_state(state.clone());
    let server = TestServer::new(app).expect("test server starts");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "Prove the spectral theorem"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert!(!body["models"]
        .as_array()
        .unwrap()
        .contains(&json!("forbidden-model")));
    assert_eq!(body["reserved_attempts"], 2);
    assert_eq!(body["dispatch_models"].as_array().unwrap().len(), 2);
    assert!(!body["dispatch_models"]
        .as_array()
        .unwrap()
        .contains(&json!("forbidden-model")));
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 1);
}

#[tokio::test]
async fn generation_endpoint_matrix_reaches_one_ballot_and_reservation_gate() {
    let mut state = build_test_state().await;
    state.routing_snapshot_preparer = Some(Arc::new(BallotSnapshotPreparer {
        catalog: CatalogSnapshot::new(vec![
            catalog_model("eligible-model", "provider-b", true),
            catalog_model("eligible-model-2", "provider-c", true),
        ]),
    }));
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    state.budget_reservation_repo = Some(budget.clone());
    let identity = ResolvedProjectIdentity {
        project_id: "project-a".to_string(),
        org_id: "org-a".to_string(),
        principal_id: "user-a".to_string(),
        auth_kind: AuthKind::Jwt,
    };
    let app = Router::new()
        .route("/v1/chat/completions", post(authorized_generation_dispatch))
        .route(
            "/v1/chat/completions/stream",
            post(authorized_generation_dispatch),
        )
        .route("/v1/completions", post(authorized_generation_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            budget_reservation_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routing_snapshot_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            classification_middleware_with_state,
        ))
        .layer(Extension(identity))
        .layer(middleware::from_fn(request_logging))
        .with_state(state);
    let server = TestServer::new(app).expect("generation matrix server starts");
    let cases = [
        (
            "/v1/chat/completions",
            json!({"model": "auto", "messages": [{"role": "user", "content": "hello"}]}),
            "chat",
            "buffered",
        ),
        (
            "/v1/chat/completions",
            json!({"model": "auto", "messages": [{"role": "user", "content": "hello"}], "stream": true}),
            "chat",
            "streaming",
        ),
        (
            "/v1/chat/completions/stream",
            json!({"model": "auto", "messages": [{"role": "user", "content": "hello"}]}),
            "chat",
            "streaming",
        ),
        (
            "/v1/completions",
            json!({"model": "auto", "prompt": "hello"}),
            "text",
            "buffered",
        ),
        (
            "/v1/completions",
            json!({"model": "auto", "prompt": "hello", "stream": true}),
            "text",
            "streaming",
        ),
    ];

    let expected_case_count = cases.len();
    for (index, (path, request, endpoint, transport)) in cases.into_iter().enumerate() {
        let response = server
            .post(path)
            .add_header("idempotency-key", format!("p0c-matrix-{index}"))
            .json(&request)
            .await;

        assert_eq!(response.status_code(), StatusCode::OK, "case {index}");
        let body: serde_json::Value = response.json();
        assert_eq!(body["endpoint"], endpoint, "case {index}");
        assert_eq!(body["transport"], transport, "case {index}");
        assert_eq!(body["ballot_entries"], 2, "case {index}");
        assert!(
            !body["reservation_id"]
                .as_str()
                .unwrap_or_default()
                .is_empty(),
            "case {index}"
        );
    }

    assert_eq!(budget.commands.lock().unwrap().len(), expected_case_count);
}

#[tokio::test]
async fn active_learned_ballot_and_hard_exclusions_are_retained_across_four_transports() {
    let mut state = build_test_state().await;
    state.routing_snapshot_preparer = Some(Arc::new(LearnedBallotSnapshotPreparer {
        catalog: CatalogSnapshot::new(vec![
            learned_catalog_model("forbidden-model", "provider-a", false),
            learned_catalog_model("learned-first", "provider-b", true),
            learned_catalog_model("learned-second", "provider-c", true),
        ]),
    }));
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    state.budget_reservation_repo = Some(budget);
    let identity = ResolvedProjectIdentity {
        project_id: "project-a".to_string(),
        org_id: "org-a".to_string(),
        principal_id: "user-a".to_string(),
        auth_kind: AuthKind::Jwt,
    };
    let app = Router::new()
        .route("/v1/chat/completions", post(learned_generation_dispatch))
        .route("/v1/completions", post(learned_generation_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            budget_reservation_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            selection_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routing_snapshot_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            classification_middleware_with_state,
        ))
        .layer(Extension(identity))
        .layer(middleware::from_fn(request_logging))
        .with_state(state);
    let server = TestServer::new(app).expect("learned generation matrix server starts");
    let cases = [
        (
            "/v1/chat/completions",
            json!({"model": "auto", "messages": [{"role": "user", "content": "Implement and compile Rust code"}], "max_tokens": 64}),
        ),
        (
            "/v1/chat/completions",
            json!({"model": "auto", "messages": [{"role": "user", "content": "Implement and compile Rust code"}], "max_tokens": 64, "stream": true}),
        ),
        (
            "/v1/completions",
            json!({"model": "auto", "prompt": "Implement and compile Rust code", "max_tokens": 64}),
        ),
        (
            "/v1/completions",
            json!({"model": "auto", "prompt": "Implement and compile Rust code", "max_tokens": 64, "stream": true}),
        ),
    ];

    let mut records = Vec::new();
    let mut predictor_headers = Vec::new();
    let mut bella_headers = Vec::new();
    for (index, (path, request)) in cases.into_iter().enumerate() {
        let response = server
            .post(path)
            .add_header("idempotency-key", format!("p2-learned-matrix-{index}"))
            .json(&request)
            .await;
        assert_eq!(response.status_code(), StatusCode::OK, "case {index}");
        predictor_headers.push([
            response
                .header("x-gaussmeridian-predictor-status")
                .to_str()
                .expect("status header")
                .to_string(),
            response
                .header("x-gaussmeridian-predictor-outcome-estimates")
                .to_str()
                .expect("outcome count header")
                .to_string(),
            response
                .header("x-gaussmeridian-predictor-cost-estimates")
                .to_str()
                .expect("cost count header")
                .to_string(),
            response
                .header("x-gaussmeridian-predictor-abstained-routes")
                .to_str()
                .expect("abstained count header")
                .to_string(),
            response
                .header("x-gaussmeridian-predictor-fallback-reason")
                .to_str()
                .expect("fallback reason header")
                .to_string(),
        ]);
        bella_headers.push([
            response
                .header("x-gaussmeridian-bella-status")
                .to_str()
                .expect("BELLA status header")
                .to_string(),
            response
                .header("x-gaussmeridian-bella-state-id")
                .to_str()
                .expect("BELLA state header")
                .to_string(),
            response
                .header("x-gaussmeridian-bella-taxonomy-version")
                .to_str()
                .expect("BELLA taxonomy header")
                .to_string(),
            response
                .header("x-gaussmeridian-bella-profiler-version")
                .to_str()
                .expect("BELLA profiler header")
                .to_string(),
            response
                .header("x-gaussmeridian-bella-critic-version")
                .to_str()
                .expect("BELLA critic header")
                .to_string(),
            response
                .header("x-gaussmeridian-bella-fallback-reason")
                .to_str()
                .expect("BELLA fallback reason header")
                .to_string(),
            response
                .header("x-gaussmeridian-bella-task-skill-count")
                .to_str()
                .expect("BELLA task-skill count header")
                .to_string(),
            response
                .header("x-gaussmeridian-bella-capable-route-count")
                .to_str()
                .expect("BELLA capable-route count header")
                .to_string(),
            response
                .header("x-gaussmeridian-snapshot-fingerprint")
                .to_str()
                .expect("snapshot fingerprint header")
                .to_string(),
            response
                .header("x-gaussmeridian-ballot-id")
                .to_str()
                .expect("ballot identity header")
                .to_string(),
        ]);
        records.push(response.json::<serde_json::Value>());
    }

    assert_eq!(records[0]["endpoint"], "chat");
    assert_eq!(records[0]["transport"], "buffered");
    assert_eq!(records[1]["endpoint"], "chat");
    assert_eq!(records[1]["transport"], "streaming");
    assert_eq!(records[2]["endpoint"], "text");
    assert_eq!(records[2]["transport"], "buffered");
    assert_eq!(records[3]["endpoint"], "text");
    assert_eq!(records[3]["transport"], "streaming");
    assert_eq!(
        predictor_headers,
        vec![
            [
                "estimated".to_string(),
                "3".to_string(),
                "3".to_string(),
                "0".to_string(),
                "none".to_string(),
            ];
            4
        ]
    );
    let expected_bella_headers = [
        "applied".to_string(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        "skills-test/v1".to_string(),
        "meridian-skill-profiler/v1".to_string(),
        "controlled-skill-critic-test/v1".to_string(),
        "none".to_string(),
        "1".to_string(),
        "2".to_string(),
    ];
    for headers in &bella_headers {
        assert_eq!(&headers[..8], expected_bella_headers.as_slice());
        assert!(!headers[8].is_empty());
        assert!(!headers[9].is_empty());
    }
    let expected_models = json!(["learned-first", "learned-second"]);
    for (record, headers) in records.into_iter().zip(bella_headers) {
        assert!(!record["ballot_id"].as_str().unwrap_or_default().is_empty());
        assert_eq!(record["ballot_id"].as_str(), Some(headers[9].as_str()));
        assert_eq!(
            record["snapshot_fingerprint"].as_str(),
            Some(headers[8].as_str())
        );
        assert_eq!(record["models"], expected_models);
        assert_eq!(record["learner_state_id"], "learner-state-active-test");
        assert_eq!(record["hard_exclusions"], json!(["forbidden-model"]));
        assert_eq!(record["learned_predictions_used"], true);
        assert_eq!(record["selection_matches_ballot"], true);
    }
}

#[tokio::test]
async fn standard_stream_flag_bypasses_response_cache() {
    let state = build_test_state().await;
    let identity = ResolvedProjectIdentity {
        project_id: "project-a".to_string(),
        org_id: "org-a".to_string(),
        principal_id: "user-a".to_string(),
        auth_kind: AuthKind::Jwt,
    };
    let app = Router::new()
        .route("/v1/chat/completions", post(cache_probe_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            cache_middleware_with_state,
        ))
        .layer(Extension(identity))
        .with_state(state.clone());
    let server = TestServer::new(app).expect("stream cache probe starts");
    let request = json!({
        "model": "auto",
        "messages": [{"role": "user", "content": "hello"}],
        "stream": true,
    });

    let first = server.post("/v1/chat/completions").json(&request).await;
    let second = server.post("/v1/chat/completions").json(&request).await;

    assert_eq!(first.status_code(), StatusCode::OK);
    assert_eq!(second.status_code(), StatusCode::OK);
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 2);
}

#[tokio::test]
async fn buffered_text_uses_the_frozen_ballot_and_native_response_shape() {
    let (state, provider_calls) = provider_boundary_state(2).await;
    let server = text_provider_boundary_server(state);

    let response = server
        .post("/v1/completions")
        .add_header("idempotency-key", "p0c-buffered-text")
        .json(&json!({
            "model": "auto",
            "prompt": "buffered text boundary test",
            "max_tokens": 64
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_p1_routing_audit_headers(&response);
    let body: serde_json::Value = response.json();
    assert_eq!(body["object"], "text_completion");
    assert_eq!(body["model"], "eligible-model");
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model"]
    );
}

#[tokio::test]
async fn provider_dispatch_does_not_invent_an_unballoted_moa_action() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let moa_calls = Arc::new(Mutex::new(0));
    state.moa_engine = Some(recording_moa_engine(moa_calls.clone()).await);
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let server = provider_boundary_server(state, true);

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "provider boundary test"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_p1_routing_audit_headers(&response);
    let ballot_id = response
        .header("x-gaussmeridian-ballot-id")
        .to_str()
        .expect("ballot id must be valid ASCII")
        .to_owned();
    assert_eq!(
        response.header("x-gaussmeridian-provider"),
        "provider-c",
        "the successful response must come from the second ballot action"
    );
    assert_eq!(
        *moa_calls.lock().unwrap(),
        0,
        "MoA is not a ballot action in P0B"
    );
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model", "eligible-model-2"]
    );
    let records = attempts.records.lock().unwrap();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|record| record.ballot_id == ballot_id));
}

#[tokio::test]
async fn provider_dispatch_uses_the_frozen_attempt_prefix_not_mutable_server_config() {
    let (state, provider_calls) = provider_boundary_state(1).await;
    let server = provider_boundary_server(state, false);

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "provider boundary test"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.header("x-gaussmeridian-provider"), "provider-c");
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model", "eligible-model-2"]
    );
}

#[tokio::test]
async fn streaming_falls_back_only_before_meaningful_output() {
    let (state, provider_calls) = provider_boundary_state(2).await;
    let dispatch_count = state.routing_metrics.clone();
    let server = provider_boundary_server(state, false);

    let response = server
        .post("/v1/chat/completions/stream")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "provider boundary test"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_p1_routing_audit_headers(&response);
    assert_eq!(response.header("content-type"), "text/event-stream");
    let body = response.text();
    assert!(body.contains("\"content\":\"ok\""));
    assert_eq!(body.matches("data: [DONE]\n\n").count(), 1);
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model", "eligible-model-2"]
    );
    assert_eq!(dispatch_count.lock().unwrap().total_requests, 1);
}

#[tokio::test]
async fn streaming_role_only_preface_does_not_commit_provider_identity() {
    let (state, provider_calls) = provider_boundary_state(2).await;
    let server = streaming_chat_provider_boundary_server(state, "role-only-error");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "role-only-error"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(!body.contains("\"role\":\"assistant\""));
    assert!(body.contains("\"content\":\"ok\""));
    assert_eq!(body.matches("data: [DONE]\n\n").count(), 1);
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model", "eligible-model-2"]
    );
}

#[tokio::test]
async fn streaming_provider_error_after_commit_never_splices_candidates() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let server = streaming_chat_provider_boundary_server(state, "postcommit-error");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "postcommit-error"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("\"content\":\"ok\""));
    assert!(body.contains("\"code\":\"provider_stream_error\""));
    assert!(!body.contains("data: [DONE]\n\n"));
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model"]
    );
    let records = attempts.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    let attempt = &records[0];
    assert_eq!(attempt.state, ProviderAttemptState::Failed);
    assert_eq!(
        attempt.transport_outcome,
        Some(TransportOutcome::ProviderError)
    );
    assert!(attempt.response_committed);
    assert_eq!(attempt.cost_status, ProviderCostStatus::Unresolved);
    assert_eq!(attempts.finalization_count(&attempt.attempt_id), 1);
}

#[tokio::test]
async fn observed_committed_stream_failure_uses_atomic_finalization() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: Some(attempts.clone()),
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let server = streaming_chat_provider_boundary_server(state, "postcommit-error-observed");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "postcommit-error-observed"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("\"code\":\"provider_stream_error\""));
    assert!(!body.contains("data: [DONE]\n\n"));
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model"]
    );
    assert_eq!(attempts.atomic_finalization_count(), 1);
    let records = attempts.records.lock().unwrap();
    assert_eq!(records[0].state, ProviderAttemptState::Failed);
    assert!(records[0].response_committed);
    assert_eq!(records[0].cost_status, ProviderCostStatus::Observed);
}

#[tokio::test]
async fn committed_stream_atomic_failure_never_leaves_a_split_attempt_write() {
    let (mut state, _) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::SettlementUnavailable,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: Some(attempts.clone()),
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let server = streaming_chat_provider_boundary_server(state, "successful-finish");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "successful-finish"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert!(!response.text().contains("data: [DONE]\n\n"));
    tokio::task::yield_now().await;
    assert!(attempts.atomic_finalization_count() >= 1);
    let records = attempts.records.lock().unwrap();
    assert_eq!(records[0].state, ProviderAttemptState::Started);
    assert!(!records[0].response_committed);
}

#[tokio::test]
async fn precommit_prefaces_are_bounded_by_capacity_and_one_absolute_deadline() {
    for scenario in ["precommit-overflow", "precommit-slow-preface"] {
        let (mut state, provider_calls) = provider_boundary_state(2).await;
        Arc::make_mut(&mut state.config).server.request_timeout = 1;
        let attempts = Arc::new(RecordingAttemptRepository::default());
        let budget = Arc::new(RecordingBudgetRepository {
            behavior: ReservationBehavior::Reserve,
            commands: Arc::new(Mutex::new(Vec::new())),
            committed_attempts: Some(attempts.clone()),
        });
        install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
        let server = streaming_chat_provider_boundary_server(state, scenario);

        let response = tokio::time::timeout(
            Duration::from_secs(3),
            server.post("/v1/chat/completions").json(&json!({
                "model": "auto",
                "messages": [{"role": "user", "content": scenario}],
                "stream": true
            })),
        )
        .await
        .expect("absolute precommit deadline bounds the request");

        assert_eq!(response.status_code(), StatusCode::OK, "{scenario}");
        let body = response.text();
        assert!(!body.contains("\"role\":\"assistant\""), "{scenario}");
        assert_eq!(body.matches("data: [DONE]\n\n").count(), 1, "{scenario}");
        assert_eq!(
            provider_calls.lock().unwrap().as_slice(),
            ["eligible-model", "eligible-model-2"],
            "{scenario}"
        );
        let records = attempts.records.lock().unwrap();
        assert_eq!(records.len(), 2, "{scenario}");
        assert!(!records[0].response_committed, "{scenario}");
        let expected_state = if scenario == "precommit-overflow" {
            ProviderAttemptState::Failed
        } else {
            ProviderAttemptState::TimedOut
        };
        assert_eq!(records[0].state, expected_state, "{scenario}");
    }
}

#[tokio::test]
async fn response_body_poll_boundary_controls_stream_commitment() {
    for (frames_to_poll, expected_committed) in [(0usize, false), (1, false), (2, true)] {
        let (mut state, _) = provider_boundary_state(2).await;
        let attempts = Arc::new(RecordingAttemptRepository::default());
        let budget = Arc::new(RecordingBudgetRepository {
            behavior: ReservationBehavior::Reserve,
            commands: Arc::new(Mutex::new(Vec::new())),
            committed_attempts: Some(attempts.clone()),
        });
        install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
        let response = raw_streaming_chat_response(state, "role-then-content").await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut body = response.into_body().into_data_stream();
        for index in 0..frames_to_poll {
            let frame = body
                .next()
                .await
                .expect("expected buffered frame")
                .expect("buffered frame is readable");
            if index == 0 {
                assert!(String::from_utf8_lossy(&frame).contains("\"role\":\"assistant\""));
            }
        }
        drop(body);
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(20)).await;

        let records = attempts.records.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].state, ProviderAttemptState::Cancelled);
        assert_eq!(records[0].response_committed, expected_committed);
    }
}

#[tokio::test]
async fn dropping_a_committed_stream_retains_observed_output_evidence() {
    let (mut state, _) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: Some(attempts.clone()),
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let response = raw_streaming_chat_response(state, "successful-finish").await;
    assert_eq!(response.status(), StatusCode::OK);

    let mut body = response.into_body().into_data_stream();
    let content = body
        .next()
        .await
        .expect("expected committed content frame")
        .expect("committed content frame is readable");
    assert!(String::from_utf8_lossy(&content).contains("\"content\":\"ok\""));
    let finish = body
        .next()
        .await
        .expect("expected observed finish frame")
        .expect("observed finish frame is readable");
    assert!(String::from_utf8_lossy(&finish).contains("\"finish_reason\":\"stop\""));
    drop(body);
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(20)).await;

    let records = attempts.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, ProviderAttemptState::Succeeded);
    assert_eq!(
        records[0].transport_outcome,
        Some(TransportOutcome::Delivered)
    );
    assert!(records[0].response_committed);
    assert_eq!(records[0].actual_output_tokens, Some(1));
    assert_eq!(records[0].finish_reason.as_deref(), Some("stop"));
    assert_eq!(records[0].output_constraint_version, None);
    assert_eq!(records[0].token_limit_compliant, Some(true));
    assert_eq!(records[0].cost_status, ProviderCostStatus::Observed);
}

#[tokio::test]
async fn cancelling_after_a_finish_only_precommit_chunk_remains_failed_and_empty() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: Some(attempts.clone()),
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let mut app = streaming_chat_provider_boundary_app(state, "finish-only-pending");
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "model": "auto",
                "messages": [{"role": "user", "content": "finish-only-pending"}],
                "stream": true
            }))
            .expect("finish-only request serializes"),
        ))
        .expect("finish-only request builds");
    let request_task = tokio::spawn(async move { tower::Service::call(&mut app, request).await });

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if provider_calls
                .lock()
                .unwrap()
                .iter()
                .any(|call| call == "finish-only-polled")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("finish-only provider chunk is observed");
    request_task.abort();
    let _ = request_task.await;

    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            let finalized = attempts.records.lock().unwrap().iter().any(|record| {
                record.attempt_no == 0 && record.state != ProviderAttemptState::Started
            });
            if finalized {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("finish-only attempt reaches a durable terminal state");

    let records = attempts.records.lock().unwrap();
    let attempt = records
        .iter()
        .find(|record| record.attempt_no == 0)
        .expect("first finish-only attempt remains durable");
    assert_eq!(attempt.state, ProviderAttemptState::Failed);
    assert_eq!(attempt.transport_outcome, Some(TransportOutcome::Empty));
    assert!(!attempt.response_committed);
    assert_eq!(attempt.actual_output_tokens, Some(1));
    assert_eq!(attempt.finish_reason.as_deref(), Some("stop"));
    assert_eq!(attempt.cost_status, ProviderCostStatus::Observed);
}

#[tokio::test]
async fn cancelling_buffered_execution_retains_evidence_observed_before_outcome_gate() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind blocking outcome webhook");
    let webhook_url = format!(
        "http://{}/validate",
        listener.local_addr().expect("read webhook address")
    );
    let (accepted_tx, accepted_rx) = tokio::sync::oneshot::channel();
    let webhook_task = tokio::spawn(async move {
        let (_socket, _) = listener.accept().await.expect("accept outcome webhook");
        let _ = accepted_tx.send(());
        futures::future::pending::<()>().await;
    });

    let (mut state, _) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let settings = ProjectSettingsExt {
        validator_type: "webhook".to_string(),
        validator_config: json!({"url": webhook_url, "timeout_ms": 30_000}),
        ..ProjectSettingsExt::default()
    };
    let mut app = provider_boundary_app(state, false, settings);
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "model": "auto",
                "messages": [{"role": "user", "content": "provider boundary test"}]
            }))
            .expect("buffered request serializes"),
        ))
        .expect("buffered request builds");
    let request_task = tokio::spawn(async move { tower::Service::call(&mut app, request).await });

    tokio::time::timeout(Duration::from_secs(2), accepted_rx)
        .await
        .expect("provider response reached outcome webhook")
        .expect("outcome webhook acceptance signal remains available");
    request_task.abort();
    let _ = request_task.await;
    webhook_task.abort();
    let _ = webhook_task.await;
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(20)).await;

    let records = attempts.records.lock().unwrap();
    assert_eq!(records.len(), 2);
    let cancelled = &records[1];
    assert_eq!(cancelled.state, ProviderAttemptState::Succeeded);
    assert_eq!(
        cancelled.transport_outcome,
        Some(TransportOutcome::Delivered)
    );
    assert!(!cancelled.response_committed);
    assert_eq!(cancelled.actual_output_tokens, Some(1));
    assert_eq!(cancelled.finish_reason.as_deref(), Some("stop"));
    assert_eq!(cancelled.output_constraint_version, None);
    assert_eq!(cancelled.token_limit_compliant, Some(true));
    assert_eq!(cancelled.cost_status, ProviderCostStatus::Observed);
}

#[tokio::test]
async fn cancelling_while_attempt_finalization_is_blocked_retries_the_exact_terminal_command() {
    let (mut state, _) = provider_boundary_state(2).await;
    let recorded_attempts = Arc::new(RecordingAttemptRepository::default());
    let blocking_attempts = Arc::new(BlockingAttemptRepository::new(recorded_attempts.clone()));
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, blocking_attempts.clone());

    let mut app = provider_boundary_app(state, false, ProjectSettingsExt::default());
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "model": "auto",
                "messages": [{"role": "user", "content": "provider boundary test"}]
            }))
            .expect("buffered request serializes"),
        ))
        .expect("buffered request builds");
    let request_task = tokio::spawn(async move { tower::Service::call(&mut app, request).await });

    tokio::time::timeout(Duration::from_secs(2), blocking_attempts.blocked.notified())
        .await
        .expect("successful attempt finalization reaches the blocking repository");
    let calls_before_abort = blocking_attempts.calls.load(Ordering::SeqCst);
    request_task.abort();
    let _ = request_task.await;

    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            let recovered = blocking_attempts.calls.load(Ordering::SeqCst) > calls_before_abort
                && recorded_attempts
                    .records
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|record| record.state == ProviderAttemptState::Succeeded);
            if recovered {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("guard drop must retry the terminal command cancelled during persistence");

    let commands = blocking_attempts.succeeded_commands.lock().unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0], commands[1]);
    let records = recorded_attempts.records.lock().unwrap();
    let record = records
        .iter()
        .find(|record| record.attempt_id == commands[0].attempt_id)
        .expect("retried attempt command reaches durable storage");
    assert_attempt_matches_finalization(record, &commands[0]);
}

#[tokio::test]
async fn cancelling_while_atomic_stream_finalization_is_blocked_retries_the_exact_command() {
    let (mut state, _) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let inner_budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: Some(attempts.clone()),
    });
    let blocking_budget = Arc::new(BlockingCommittedStreamRepository::new(inner_budget));
    install_budget_lifecycle_with_attempts(&mut state, blocking_budget.clone(), attempts.clone());

    let response = raw_streaming_chat_response(state, "successful-finish").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body_task = tokio::spawn(async move {
        let mut body = response.into_body().into_data_stream();
        while body.next().await.is_some() {}
    });

    tokio::time::timeout(Duration::from_secs(2), blocking_budget.blocked.notified())
        .await
        .expect("committed stream finalization reaches the blocking repository");
    let calls_before_abort = blocking_budget.calls.load(Ordering::SeqCst);
    body_task.abort();
    let _ = body_task.await;

    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            let recovered = blocking_budget.calls.load(Ordering::SeqCst) > calls_before_abort
                && attempts.records.lock().unwrap().iter().any(|record| {
                    record.state == ProviderAttemptState::Succeeded && record.response_committed
                });
            if recovered {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("guard drop must retry the atomic command cancelled during persistence");

    let commands = blocking_budget.commands.lock().unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0], commands[1]);
    let command = &commands[0];

    let attempt_records = attempts.records.lock().unwrap();
    let attempt = attempt_records
        .iter()
        .find(|record| record.attempt_id == command.attempt.attempt_id)
        .expect("retried atomic attempt reaches durable storage");
    assert_eq!(
        attempt.reservation_id.as_deref(),
        Some(command.reservation_id.as_str())
    );
    assert_attempt_matches_finalization(attempt, &command.attempt);

    let reservations = blocking_budget.completed_reservations.lock().unwrap();
    assert_eq!(reservations.len(), 1);
    let reservation = &reservations[0];
    assert_eq!(reservation.reservation_id, command.reservation_id);
    assert_eq!(reservation.state, "settled");
    assert_eq!(
        reservation.actual_provider_cost,
        Some(
            command.attempt.input_cost
                + command.attempt.output_cost
                + command.attempt.reasoning_cost
                + command.attempt.tools_cost
                + command.attempt.other_cost
        )
    );
    assert_eq!(
        reservation.finalization_id.as_deref(),
        Some(command.attempt.finalization_id.as_str())
    );
    assert_eq!(
        reservation.terminal_outcome.as_deref(),
        Some(command.terminal.outcome.as_str())
    );
    assert_eq!(
        reservation.response_committed,
        Some(command.terminal.response_committed)
    );
    assert_eq!(
        reservation.committed_attempt_id,
        command.terminal.committed_attempt_id
    );
    assert_eq!(
        reservation.customer_charge,
        Some(command.terminal.customer_charge)
    );
    assert_eq!(reservation.finalized_at, Some(command.attempt.finalized_at));
}

#[tokio::test]
async fn canonical_text_stream_uses_native_sse_and_ballot_fallback() {
    let (state, provider_calls) = provider_boundary_state(2).await;
    let server = streaming_text_provider_boundary_server(state);

    let response = server
        .post("/v1/completions")
        .json(&json!({
            "model": "auto",
            "prompt": "text stream boundary test",
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_p1_routing_audit_headers(&response);
    assert_eq!(response.header("content-type"), "text/event-stream");
    let body = response.text();
    assert!(body.contains("\"object\":\"text_completion\""));
    assert!(body.contains("\"text\":\"ok\""));
    assert!(!body.contains("chat.completion.chunk"));
    assert_eq!(body.matches("data: [DONE]\n\n").count(), 1);
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model", "eligible-model-2"]
    );
}

#[tokio::test]
async fn stream_premature_eof_after_commit_finalizes_stream_failed_with_unresolved_cost() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: Some(attempts.clone()),
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let server = streaming_chat_provider_boundary_server(state, "successful-eof");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "successful-eof"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("\"content\":\"ok\""));
    assert_eq!(body.matches("data: [DONE]\n\n").count(), 0);
    assert!(body.contains("\"code\":\"provider_stream_incomplete\""));
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model"]
    );
    let records = attempts.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, ProviderAttemptState::Failed);
    assert_eq!(records[0].transport_outcome, Some(TransportOutcome::Empty));
    assert!(records[0].response_committed);
    assert_eq!(records[0].cost_status, ProviderCostStatus::Unresolved);
    assert_eq!(attempts.finalization_count(&records[0].attempt_id), 1);
}

#[tokio::test]
async fn stream_explicit_finish_settles_observed_cost_once() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: Some(attempts.clone()),
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let server = streaming_chat_provider_boundary_server(state, "successful-finish");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "successful-finish"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert_eq!(body.matches("data: [DONE]\n\n").count(), 1);
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model"]
    );
    let records = attempts.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, ProviderAttemptState::Succeeded);
    assert_eq!(
        records[0].transport_outcome,
        Some(TransportOutcome::Delivered)
    );
    assert!(records[0].response_committed);
    assert_eq!(records[0].cost_status, ProviderCostStatus::Observed);
    assert!(records[0].provider_cost_incurred > 0.0);
    assert_eq!(attempts.finalization_count(&records[0].attempt_id), 1);
}

#[tokio::test]
async fn stream_timeout_after_commit_is_terminal_without_fallback() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    Arc::make_mut(&mut state.config).server.request_timeout = 1;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let server = streaming_chat_provider_boundary_server(state, "postcommit-timeout");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "postcommit-timeout"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("\"code\":\"provider_stream_timeout\""));
    assert!(!body.contains("data: [DONE]\n\n"));
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model"]
    );
    let records = attempts.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, ProviderAttemptState::TimedOut);
    assert_eq!(
        records[0].transport_outcome,
        Some(TransportOutcome::Timeout)
    );
    assert!(records[0].response_committed);
    assert_eq!(records[0].cost_status, ProviderCostStatus::Unresolved);
    assert_eq!(attempts.finalization_count(&records[0].attempt_id), 1);
}

#[tokio::test]
async fn stream_provider_error_before_output_may_fallback() {
    let (state, provider_calls) = provider_boundary_state(2).await;
    let server = streaming_chat_provider_boundary_server(state, "precommit-error");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "precommit-error"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.text().matches("data: [DONE]\n\n").count(), 1);
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model", "eligible-model-2"]
    );
}

#[tokio::test]
async fn stream_ballot_exhaustion_retains_unresolved_exposure_and_returns_typed_502() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository::default());
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts.clone());
    let server = streaming_chat_provider_boundary_server(state, "all-precommit-error");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "all-precommit-error"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        response.json::<serde_json::Value>()["error"]["code"],
        "all_providers_failed"
    );
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model", "eligible-model-2"]
    );
    let records = attempts.records.lock().unwrap();
    assert_eq!(records.len(), 2);
    for attempt in records.iter() {
        assert_eq!(attempt.state, ProviderAttemptState::Failed);
        assert_eq!(
            attempt.transport_outcome,
            Some(TransportOutcome::ProviderError)
        );
        assert!(!attempt.response_committed);
        assert_eq!(attempt.cost_status, ProviderCostStatus::Unresolved);
        assert_eq!(attempts.finalization_count(&attempt.attempt_id), 1);
    }
}

#[tokio::test]
async fn stream_precommit_persistence_failure_blocks_next_candidate_dispatch() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let attempts = Arc::new(RecordingAttemptRepository {
        fail_finalization: true,
        ..RecordingAttemptRepository::default()
    });
    let budget = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    install_budget_lifecycle_with_attempts(&mut state, budget, attempts);
    let server = streaming_chat_provider_boundary_server(state, "precommit-error");

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "precommit-error"}],
            "stream": true
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        provider_calls.lock().unwrap().as_slice(),
        ["eligible-model"]
    );
}

async fn delayed_dispatch(State(state): State<AppState>) -> StatusCode {
    tokio::time::sleep(Duration::from_millis(1_200)).await;
    state.routing_metrics.lock().unwrap().total_requests += 1;
    StatusCode::OK
}

#[tokio::test]
async fn provider_dispatch_requires_the_authoritative_budget_repository() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    state.budget_reservation_repo = None;
    state.provider_attempt_repo = None;
    state.attempt_lifecycle = None;
    let server = budgeted_provider_boundary_server(state);

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "budget boundary test"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response.json::<serde_json::Value>()["error"]["code"],
        "service_unavailable"
    );
    assert!(provider_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn budget_repository_failure_is_sanitized_before_provider_dispatch() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let commands = Arc::new(Mutex::new(Vec::new()));
    install_budget_lifecycle(
        &mut state,
        Arc::new(RecordingBudgetRepository {
            behavior: ReservationBehavior::Unavailable,
            commands: commands.clone(),
            committed_attempts: None,
        }),
    );
    let server = budgeted_provider_boundary_server(state);

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "budget boundary test"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response.json::<serde_json::Value>()["error"]["code"],
        "service_unavailable"
    );
    assert_eq!(commands.lock().unwrap().len(), 1);
    assert!(provider_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn insufficient_budget_rejects_the_frozen_trajectory_before_dispatch() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let commands = Arc::new(Mutex::new(Vec::new()));
    install_budget_lifecycle(
        &mut state,
        Arc::new(RecordingBudgetRepository {
            behavior: ReservationBehavior::Insufficient,
            commands: commands.clone(),
            committed_attempts: None,
        }),
    );
    let server = budgeted_provider_boundary_server(state);

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "budget boundary test"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::PAYMENT_REQUIRED);
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["error"]["type"], "budget_exceeded");
    assert_eq!(body["error"]["code"], "payment_required");
    assert!(!body["error"]["request_id"].as_str().unwrap().is_empty());
    assert_eq!(commands.lock().unwrap().len(), 1);
    assert!(provider_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn frozen_trajectory_is_reserved_before_the_first_provider_call() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let expected_fingerprint = persisted_ballot_input(2).frozen.fingerprint;
    let expected_expiry_seconds = i64::try_from(
        state.config.server.request_timeout + state.config.server.graceful_shutdown_timeout,
    )
    .unwrap();
    let commands = Arc::new(Mutex::new(Vec::new()));
    let concrete_attempts = Arc::new(RecordingAttemptRepository::default());
    install_budget_lifecycle_with_attempts(
        &mut state,
        Arc::new(RecordingBudgetRepository {
            behavior: ReservationBehavior::Reserve,
            commands: commands.clone(),
            committed_attempts: Some(concrete_attempts.clone()),
        }),
        concrete_attempts.clone(),
    );
    let server = budgeted_provider_boundary_server(state);

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "budget boundary test"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(provider_calls.lock().unwrap().len(), 2);
    let reservation_id = {
        let commands = commands.lock().unwrap();
        assert_eq!(commands.len(), 1);
        let command = &commands[0];
        assert!(!command.request_id.is_empty());
        assert_eq!(command.reservation_id.len(), 64);
        assert_eq!(command.project_id, "project-a");
        assert_eq!(command.snapshot_fingerprint, expected_fingerprint);
        assert_eq!(
            command.period_key,
            command.created_at.format("%Y-%m").to_string()
        );
        assert!((command.amount - 0.001384).abs() < 1e-12);
        assert_eq!(
            (command.expires_at - command.created_at).num_seconds(),
            expected_expiry_seconds
        );
        command.reservation_id.clone()
    };
    let attempts = concrete_attempts
        .list_for_reservation(&reservation_id)
        .await
        .expect("read finalized provider attempts");
    assert_eq!(attempts.len(), 2);
    assert!(!attempts[0].response_committed);
    assert!(attempts[1].response_committed);
}

#[tokio::test]
async fn settlement_failure_never_marks_the_selected_attempt_committed() {
    let (mut state, provider_calls) = provider_boundary_state(1).await;
    let commands = Arc::new(Mutex::new(Vec::new()));
    let concrete_attempts = Arc::new(RecordingAttemptRepository::default());
    install_budget_lifecycle_with_attempts(
        &mut state,
        Arc::new(RecordingBudgetRepository {
            behavior: ReservationBehavior::SettlementUnavailable,
            commands: commands.clone(),
            committed_attempts: Some(concrete_attempts.clone()),
        }),
        concrete_attempts.clone(),
    );
    let server = budgeted_provider_boundary_server(state);

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "settlement failure test"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(provider_calls.lock().unwrap().len(), 2);
    let reservation_id = commands
        .lock()
        .unwrap()
        .first()
        .expect("reservation command recorded")
        .reservation_id
        .clone();
    let attempts = concrete_attempts
        .list_for_reservation(&reservation_id)
        .await
        .expect("read attempt after injected settlement failure");
    assert_eq!(attempts.len(), 2);
    assert!(attempts.iter().all(|attempt| !attempt.response_committed));
}

#[tokio::test]
async fn project_scoped_idempotency_replay_never_dispatches_twice() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let commands = Arc::new(Mutex::new(Vec::new()));
    install_budget_lifecycle(
        &mut state,
        Arc::new(RecordingBudgetRepository {
            behavior: ReservationBehavior::ReserveThenReplay,
            commands: commands.clone(),
            committed_attempts: None,
        }),
    );
    let server = budgeted_provider_boundary_server(state);
    let body = json!({
        "model": "auto",
        "messages": [{"role": "user", "content": "budget boundary test"}]
    });

    let first = server
        .post("/v1/chat/completions")
        .add_header("idempotency-key", "logical-request-a")
        .json(&body)
        .await;
    let second = server
        .post("/v1/chat/completions")
        .add_header("idempotency-key", "logical-request-a")
        .json(&body)
        .await;

    assert_eq!(first.status_code(), StatusCode::OK);
    assert_eq!(second.status_code(), StatusCode::CONFLICT);
    let conflict = second.json::<serde_json::Value>();
    assert_eq!(conflict["error"]["code"], "idempotency_conflict");
    assert_eq!(provider_calls.lock().unwrap().len(), 2);
    let commands = commands.lock().unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].reservation_id, commands[1].reservation_id);
    assert_eq!(commands[0].idempotency_key, "logical-request-a");
    assert_eq!(commands[1].idempotency_key, "logical-request-a");
    assert_ne!(commands[0].request_id, commands[1].request_id);
}

#[tokio::test]
async fn same_key_different_economics_returns_typed_conflict_without_redispatch() {
    let (mut state, provider_calls) = provider_boundary_state(2).await;
    let commands = Arc::new(Mutex::new(Vec::new()));
    install_budget_lifecycle(
        &mut state,
        Arc::new(RecordingBudgetRepository {
            behavior: ReservationBehavior::ReserveThenConflict,
            commands: commands.clone(),
            committed_attempts: None,
        }),
    );
    let server = budgeted_provider_boundary_server(state);
    let body = json!({
        "model": "auto",
        "messages": [{"role": "user", "content": "budget collision test"}]
    });

    let first = server
        .post("/v1/chat/completions")
        .add_header("idempotency-key", "logical-request-collision")
        .json(&body)
        .await;
    let collision = server
        .post("/v1/chat/completions")
        .add_header("idempotency-key", "logical-request-collision")
        .json(&body)
        .await;

    assert_eq!(first.status_code(), StatusCode::OK);
    assert_eq!(collision.status_code(), StatusCode::CONFLICT);
    assert_eq!(
        collision.json::<serde_json::Value>()["error"]["code"],
        "idempotency_conflict"
    );
    assert_eq!(provider_calls.lock().unwrap().len(), 2);
    assert_eq!(commands.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn expiry_recovery_releases_unused_reservations_and_retains_unknown_liability() {
    let observed_at = Utc::now();
    let expired_command = |reservation_id: &str| ReserveBudgetCommand {
        reservation_id: reservation_id.to_string(),
        request_id: format!("request-{reservation_id}"),
        idempotency_key: format!("key-{reservation_id}"),
        project_id: "project-a".to_string(),
        snapshot_fingerprint: "a".repeat(64),
        period_key: "2026-07".to_string(),
        amount: 1.0,
        expires_at: observed_at - chrono::Duration::seconds(1),
        created_at: observed_at - chrono::Duration::minutes(1),
    };
    let commands = Arc::new(Mutex::new(vec![
        expired_command("expired-unused"),
        expired_command("expired-unresolved"),
    ]));
    let budget: Arc<dyn BudgetReservationRepositoryTrait> = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands,
        committed_attempts: None,
    });
    let concrete_attempts = Arc::new(RecordingAttemptRepository::default());
    concrete_attempts
        .start(StartProviderAttemptCommand {
            attempt_id: "expired-attempt".to_string(),
            reservation_id: "expired-unresolved".to_string(),
            request_id: "request-expired-unresolved".to_string(),
            project_id: "project-a".to_string(),
            snapshot_fingerprint: "a".repeat(64),
            ballot_id: "ballot-expired".to_string(),
            attempt_no: 0,
            provider_id: "provider-b".to_string(),
            model_id: "eligible-model".to_string(),
            requested_output_tokens: 64,
            output_constraint_version: None,
            price_version: "price-v1".to_string(),
            catalog_version: "catalog-v1".to_string(),
            policy_version: "policy-v1".to_string(),
            model_version: "model-v1".to_string(),
            started_at: observed_at - chrono::Duration::seconds(30),
        })
        .await
        .expect("start abandoned attempt");
    let attempts: Arc<dyn ProviderAttemptRepositoryTrait> = concrete_attempts.clone();

    let report = reconcile_expired_reservations(budget, attempts, observed_at, 10)
        .await
        .expect("run expiry recovery");

    assert_eq!(report.examined, 2);
    assert_eq!(report.released_without_dispatch, 1);
    assert_eq!(report.retained_unresolved, 1);
    let records = concrete_attempts
        .list_for_reservation("expired-unresolved")
        .await
        .expect("read recovered attempt");
    assert_eq!(records[0].state, ProviderAttemptState::TimedOut);
    assert_eq!(records[0].cost_status, ProviderCostStatus::Unresolved);
}

#[tokio::test]
async fn expiry_recovery_settles_a_resolved_committed_stream_as_delivered() {
    let observed_at = Utc::now();
    let reservation_id = "expired-committed-stream";
    let ballot_id = "ballot-committed-stream";
    let command = ReserveBudgetCommand {
        reservation_id: reservation_id.to_string(),
        request_id: "request-committed-stream".to_string(),
        idempotency_key: "key-committed-stream".to_string(),
        project_id: "project-a".to_string(),
        snapshot_fingerprint: "a".repeat(64),
        period_key: "2026-07".to_string(),
        amount: 1.0,
        expires_at: observed_at - chrono::Duration::seconds(1),
        created_at: observed_at - chrono::Duration::minutes(1),
    };
    let attempts = Arc::new(RecordingAttemptRepository::default());
    attempts
        .start(StartProviderAttemptCommand {
            attempt_id: "committed-stream-attempt".to_string(),
            reservation_id: reservation_id.to_string(),
            request_id: command.request_id.clone(),
            project_id: command.project_id.clone(),
            snapshot_fingerprint: command.snapshot_fingerprint.clone(),
            ballot_id: ballot_id.to_string(),
            attempt_no: 0,
            provider_id: "provider-b".to_string(),
            model_id: "eligible-model".to_string(),
            requested_output_tokens: 64,
            output_constraint_version: None,
            price_version: "price-v1".to_string(),
            catalog_version: "catalog-v1".to_string(),
            policy_version: "policy-v1".to_string(),
            model_version: "model-v1".to_string(),
            started_at: observed_at - chrono::Duration::seconds(30),
        })
        .await
        .expect("start committed stream attempt");
    let identity = TrajectoryIdentity::from_persisted(
        reservation_id.to_string(),
        ballot_id.to_string(),
        command.project_id.clone(),
        command.snapshot_fingerprint.clone(),
    );
    attempts
        .finalize(FinalizeProviderAttemptCommand {
            attempt_id: "committed-stream-attempt".to_string(),
            finalization_id: identity.finalization_id,
            terminal_state: gaussmeridian_db::ProviderAttemptTerminalState::Succeeded,
            transport_outcome: TransportOutcome::Delivered,
            response_committed: true,
            actual_output_tokens: None,
            finish_reason: None,
            output_constraint_version: None,
            token_limit_compliant: None,
            cost_status: ProviderCostStatus::Observed,
            input_cost: 0.1,
            output_cost: 0.2,
            reasoning_cost: 0.0,
            tools_cost: 0.0,
            other_cost: 0.0,
            error_code: None,
            finalized_at: observed_at - chrono::Duration::seconds(20),
        })
        .await
        .expect("finalize committed stream attempt");
    let budget: Arc<dyn BudgetReservationRepositoryTrait> = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(vec![command])),
        committed_attempts: Some(attempts.clone()),
    });
    let attempt_repo: Arc<dyn ProviderAttemptRepositoryTrait> = attempts;

    let report = reconcile_expired_reservations(budget, attempt_repo, observed_at, 10)
        .await
        .expect("recover committed stream settlement");

    assert_eq!(report.examined, 1);
    assert_eq!(report.settled_committed, 1);
    assert_eq!(report.expired_resolved, 0);
}

#[tokio::test]
async fn expiry_recovery_settles_a_resolved_committed_stream_failure() {
    let observed_at = Utc::now();
    let reservation_id = "expired-committed-stream-failure";
    let ballot_id = "ballot-committed-stream-failure";
    let command = ReserveBudgetCommand {
        reservation_id: reservation_id.to_string(),
        request_id: "request-committed-stream-failure".to_string(),
        idempotency_key: "key-committed-stream-failure".to_string(),
        project_id: "project-a".to_string(),
        snapshot_fingerprint: "a".repeat(64),
        period_key: "2026-07".to_string(),
        amount: 1.0,
        expires_at: observed_at - chrono::Duration::seconds(1),
        created_at: observed_at - chrono::Duration::minutes(1),
    };
    let attempts = Arc::new(RecordingAttemptRepository::default());
    attempts
        .start(StartProviderAttemptCommand {
            attempt_id: "committed-stream-failure-attempt".to_string(),
            reservation_id: reservation_id.to_string(),
            request_id: command.request_id.clone(),
            project_id: command.project_id.clone(),
            snapshot_fingerprint: command.snapshot_fingerprint.clone(),
            ballot_id: ballot_id.to_string(),
            attempt_no: 0,
            provider_id: "provider-b".to_string(),
            model_id: "eligible-model".to_string(),
            requested_output_tokens: 64,
            output_constraint_version: None,
            price_version: "price-v1".to_string(),
            catalog_version: "catalog-v1".to_string(),
            policy_version: "policy-v1".to_string(),
            model_version: "model-v1".to_string(),
            started_at: observed_at - chrono::Duration::seconds(30),
        })
        .await
        .expect("start committed stream failure attempt");
    let identity = TrajectoryIdentity::from_persisted(
        reservation_id.to_string(),
        ballot_id.to_string(),
        command.project_id.clone(),
        command.snapshot_fingerprint.clone(),
    );
    attempts
        .finalize(FinalizeProviderAttemptCommand {
            attempt_id: "committed-stream-failure-attempt".to_string(),
            finalization_id: identity.finalization_id,
            terminal_state: gaussmeridian_db::ProviderAttemptTerminalState::Failed,
            transport_outcome: TransportOutcome::ProviderError,
            response_committed: true,
            actual_output_tokens: None,
            finish_reason: None,
            output_constraint_version: None,
            token_limit_compliant: None,
            cost_status: ProviderCostStatus::Observed,
            input_cost: 0.1,
            output_cost: 0.2,
            reasoning_cost: 0.0,
            tools_cost: 0.0,
            other_cost: 0.0,
            error_code: Some("provider_error".to_string()),
            finalized_at: observed_at - chrono::Duration::seconds(20),
        })
        .await
        .expect("finalize committed stream failure attempt");
    let budget: Arc<dyn BudgetReservationRepositoryTrait> = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(vec![command])),
        committed_attempts: Some(attempts.clone()),
    });
    let attempt_repo: Arc<dyn ProviderAttemptRepositoryTrait> = attempts;

    let report = reconcile_expired_reservations(budget, attempt_repo, observed_at, 10)
        .await
        .expect("recover committed stream-failure settlement");

    assert_eq!(report.examined, 1);
    assert_eq!(report.settled_committed, 1);
    assert_eq!(report.failed, 0);
}

#[tokio::test]
async fn dropping_a_committed_dispatch_guard_persists_cancellation_evidence() {
    let budget: Arc<dyn BudgetReservationRepositoryTrait> = Arc::new(RecordingBudgetRepository {
        behavior: ReservationBehavior::Reserve,
        commands: Arc::new(Mutex::new(Vec::new())),
        committed_attempts: None,
    });
    let concrete_attempts = Arc::new(RecordingAttemptRepository::default());
    let attempts: Arc<dyn ProviderAttemptRepositoryTrait> = concrete_attempts.clone();
    let lifecycle = AttemptLifecycle::new(budget, attempts);
    let identity = TrajectoryIdentity {
        trajectory_id: "cancelled-trajectory".to_string(),
        ballot_id: "cancelled-ballot".to_string(),
        finalization_id: "cancelled-finalization".to_string(),
        project_id: "project-a".to_string(),
        snapshot_fingerprint: "a".repeat(64),
    };
    let context = AttemptContext {
        request_id: "cancelled-request".to_string(),
        project_id: "project-a".to_string(),
        snapshot_fingerprint: "a".repeat(64),
        price_version: "price-v1".to_string(),
        catalog_version: "catalog-v1".to_string(),
        policy_version: "policy-v1".to_string(),
        output_constraint_version: None,
    };
    let candidate = BallotCandidateProjection {
        model_name: "eligible-model".to_string(),
        provider_name: "provider-b".to_string(),
        tier: "advanced".to_string(),
        score: 0.9,
        output_token_budget: 64,
        input_per_million: 1.0,
        output_per_million: 2.0,
        fixed_cost_upper_bound: 0.0,
        model_version: "model-v1".to_string(),
    };

    let mut guard = lifecycle
        .start(&identity, &context, 0, &candidate, Utc::now())
        .await
        .expect("persist started attempt");
    guard.mark_response_committed();
    drop(guard);
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(10)).await;

    let records = concrete_attempts
        .list_for_reservation("cancelled-trajectory")
        .await
        .expect("read cancellation evidence");
    assert_eq!(records[0].state, ProviderAttemptState::Cancelled);
    assert_eq!(
        records[0].transport_outcome,
        Some(TransportOutcome::Cancelled)
    );
    assert!(records[0].response_committed);
    assert_eq!(records[0].cost_status, ProviderCostStatus::Unresolved);
    assert_eq!(
        concrete_attempts.finalization_count(&records[0].attempt_id),
        1
    );
}

#[tokio::test]
async fn recording_attempt_repository_rejects_a_changed_output_constraint_version() {
    let observed_at = Utc::now();
    let attempts = RecordingAttemptRepository::default();
    attempts
        .start(StartProviderAttemptCommand {
            attempt_id: "immutable-constraint-attempt".to_string(),
            reservation_id: "immutable-constraint-reservation".to_string(),
            request_id: "immutable-constraint-request".to_string(),
            project_id: "project-a".to_string(),
            snapshot_fingerprint: "a".repeat(64),
            ballot_id: "immutable-constraint-ballot".to_string(),
            attempt_no: 0,
            provider_id: "provider-b".to_string(),
            model_id: "eligible-model".to_string(),
            requested_output_tokens: 64,
            output_constraint_version: Some("constraint-v1".to_string()),
            price_version: "price-v1".to_string(),
            catalog_version: "catalog-v1".to_string(),
            policy_version: "policy-v1".to_string(),
            model_version: "model-v1".to_string(),
            started_at: observed_at,
        })
        .await
        .expect("start attempt with immutable constraint version");

    let error = attempts
        .finalize(FinalizeProviderAttemptCommand {
            attempt_id: "immutable-constraint-attempt".to_string(),
            finalization_id: "immutable-constraint-finalization".to_string(),
            terminal_state: gaussmeridian_db::ProviderAttemptTerminalState::Succeeded,
            transport_outcome: TransportOutcome::Delivered,
            response_committed: true,
            actual_output_tokens: Some(1),
            finish_reason: Some("stop".to_string()),
            output_constraint_version: Some("constraint-v2".to_string()),
            token_limit_compliant: Some(true),
            cost_status: ProviderCostStatus::Observed,
            input_cost: 0.0,
            output_cost: 0.0,
            reasoning_cost: 0.0,
            tools_cost: 0.0,
            other_cost: 0.0,
            error_code: None,
            finalized_at: observed_at,
        })
        .await
        .expect_err("changed constraint version must conflict");
    assert!(matches!(
        error,
        DatabaseError::IdempotencyConflict {
            entity: "provider_attempt_output_constraint",
            key
        } if key == "immutable-constraint-attempt"
    ));

    let records = attempts
        .list_for_reservation("immutable-constraint-reservation")
        .await
        .expect("read unchanged started attempt");
    assert_eq!(records[0].state, ProviderAttemptState::Started);
    assert_eq!(
        records[0].output_constraint_version.as_deref(),
        Some("constraint-v1")
    );
}

#[tokio::test]
async fn configured_request_deadline_cancels_downstream_execution() {
    let mut state = build_test_state().await;
    Arc::make_mut(&mut state.config).server.request_timeout = 1;
    let completed = state.routing_metrics.clone();
    let app = Router::new()
        .route("/deadline", post(delayed_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_deadline_middleware_with_state,
        ))
        .layer(middleware::from_fn(request_logging))
        .with_state(state);
    let server = TestServer::new(app).expect("deadline test server starts");

    let response = server.post("/deadline").await;

    assert_eq!(response.status_code(), StatusCode::REQUEST_TIMEOUT);
    let request_id = response
        .header("x-request-id")
        .to_str()
        .expect("request ID response header is ASCII")
        .to_string();
    assert!(!request_id.is_empty());
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["error"]["code"], "request_timeout");
    assert_eq!(body["error"]["request_id"], request_id);
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(completed.lock().unwrap().total_requests, 0);
}

#[tokio::test]
async fn unscoped_api_key_is_rejected_before_generation_dispatch() {
    let state = build_test_state().await;
    let server = test_server(state.clone());

    let response = server
        .post("/v1/chat/completions")
        .add_header("x-api-key", "legacy-unscoped-key")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "Explain spectral clustering"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["code"], "project_scope_required");
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 0);
}

#[tokio::test]
async fn ambiguous_jwt_project_is_rejected_before_generation_dispatch() {
    let mut state = build_test_state().await;
    state.routing_identity_resolver = Some(Arc::new(StubIdentityResolver {
        result: Err(IdentityError::ProjectContextRequired),
    }));
    let server = test_server(state.clone());

    let response = server
        .post("/v1/chat/completions")
        .add_header("authorization", format!("Bearer {}", jwt_for("user-multi")))
        .json(&json!({"model": "auto", "messages": [{"role": "user", "content": "Hi"}]}))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["code"], "project_context_required");
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 0);
}

#[tokio::test]
async fn explicit_jwt_project_outside_membership_is_rejected_before_dispatch() {
    let mut state = build_test_state().await;
    state.routing_identity_resolver = Some(Arc::new(StubIdentityResolver {
        result: Err(IdentityError::ProjectNotAccessible),
    }));
    let server = test_server(state.clone());

    let response = server
        .post("/v1/chat/completions")
        .add_header("authorization", format!("Bearer {}", jwt_for("user-a")))
        .add_header("x-project-id", "project-b")
        .json(&json!({"model": "auto", "messages": [{"role": "user", "content": "Hi"}]}))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["code"], "project_access_denied");
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 0);
}

#[tokio::test]
async fn identity_store_outage_is_sanitized_and_dispatches_nothing() {
    let mut state = build_test_state().await;
    state.routing_identity_resolver = Some(Arc::new(StubIdentityResolver {
        result: Err(IdentityError::AuthoritativeStateUnavailable),
    }));
    let server = test_server(state.clone());

    let response = server
        .post("/v1/chat/completions")
        .add_header("authorization", format!("Bearer {}", jwt_for("user-a")))
        .json(&json!({"model": "auto", "messages": [{"role": "user", "content": "Hi"}]}))
        .await;

    assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["code"], "service_unavailable");
    assert_eq!(
        body["error"]["message"],
        "Service is temporarily unavailable"
    );
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 0);
}

#[tokio::test]
async fn identical_prompts_are_never_shared_across_project_cache_boundaries() {
    let mut state = build_test_state().await;
    state.routing_identity_resolver = Some(Arc::new(RequestedProjectResolver));
    let app = Router::new()
        .route("/v1/chat/completions", post(identity_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            cache_middleware_with_state,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware_with_state,
        ))
        .with_state(state.clone());
    let server = TestServer::new(app).expect("test server starts");
    let body = json!({
        "model": "auto",
        "messages": [{"role": "user", "content": "Same prompt, different project"}]
    });
    let token = jwt_for("user-a");

    let first = server
        .post("/v1/chat/completions")
        .add_header("authorization", format!("Bearer {token}"))
        .add_header("x-project-id", "project-a")
        .json(&body)
        .await;
    let second = server
        .post("/v1/chat/completions")
        .add_header("authorization", format!("Bearer {token}"))
        .add_header("x-project-id", "project-b")
        .json(&body)
        .await;

    assert_eq!(first.json::<serde_json::Value>()["project_id"], "project-a");
    assert_eq!(
        second.json::<serde_json::Value>()["project_id"],
        "project-b"
    );
    assert_eq!(state.routing_metrics.lock().unwrap().total_requests, 2);
}
