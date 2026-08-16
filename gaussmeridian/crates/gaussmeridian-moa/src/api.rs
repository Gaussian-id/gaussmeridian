//! MoA REST API implementation
//! 
//! This module provides REST API endpoints for the MoA system, including
//! request processing, metrics, health checks, and agent management.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error, instrument};
use utoipa::OpenApi;
use utoipa::openapi::security::{SecurityScheme, ApiKey, ApiKeyValue};

use crate::{
    engine::MoaEngine,
    error::{MoaError, MoaResult},
    models::{MoaRequest, MoaResponse, AgentResponse},
    health::{HealthStatus as MoaHealthStatus, DetailedHealthStatus},
};

/// API state containing the MoA engine
#[derive(Clone)]
pub struct ApiState {
    pub engine: Arc<MoaEngine>,
}

/// Request DTO for processing
#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessRequestDto {
    pub query: String,
    pub context: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Response DTO for processing
#[derive(Debug, Serialize)]
pub struct ProcessResponseDto {
    pub id: String,
    pub content: String,
    pub confidence: f32,
    pub agent_responses: Vec<AgentResponseDto>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metrics: ResponseMetricsDto,
}

/// Agent response DTO
#[derive(Debug, Serialize)]
pub struct AgentResponseDto {
    pub id: String,
    pub agent_id: String,
    pub content: String,
    pub confidence: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metrics: ResponseMetricsDto,
}

/// Response metrics DTO
#[derive(Debug, Serialize)]
pub struct ResponseMetricsDto {
    pub latency_ms: u64,
    pub tokens_used: u32,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Health status DTO
#[derive(Debug, Serialize)]
pub struct HealthStatusDto {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub components: std::collections::HashMap<String, ComponentStatusDto>,
}

/// Component status DTO
#[derive(Debug, Serialize)]
pub struct ComponentStatusDto {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Metrics value DTO
#[derive(Debug, Serialize)]
pub struct MetricValueDto {
    pub name: String,
    pub value: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Agent status DTO
#[derive(Debug, Serialize)]
pub struct AgentStatusDto {
    pub agent_id: String,
    pub status: String,
    pub metrics: Option<AgentMetricsDto>,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Agent metrics DTO
#[derive(Debug, Serialize)]
pub struct AgentMetricsDto {
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub total_requests: u64,
    pub total_errors: u64,
}

/// Error response DTO
#[derive(Debug, Serialize)]
pub struct ErrorResponseDto {
    pub error: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl From<MoaResponse> for ProcessResponseDto {
    fn from(response: MoaResponse) -> Self {
        ProcessResponseDto {
            id: response.id.to_string(),
            content: response.content,
            confidence: response.confidence,
            agent_responses: response.agent_responses.into_iter()
                .map(|ar| AgentResponseDto {
                    id: ar.id,
                    agent_id: ar.agent_id,
                    content: ar.content,
                    confidence: ar.confidence,
                    timestamp: ar.timestamp,
                    metrics: ResponseMetricsDto {
                        latency_ms: ar.metrics.latency_ms,
                        tokens_used: ar.metrics.tokens_used,
                        prompt_tokens: ar.metrics.prompt_tokens,
                        completion_tokens: ar.metrics.completion_tokens,
                    },
                })
                .collect(),
            timestamp: response.timestamp,
            metrics: ResponseMetricsDto {
                latency_ms: response.metrics.latency_ms,
                tokens_used: response.metrics.tokens_used,
                prompt_tokens: response.metrics.prompt_tokens,
                completion_tokens: response.metrics.completion_tokens,
            },
        }
    }
}

/// Process a request through the MOA system
#[utoipa::path(
    post,
    path = "/api/v1/process",
    request_body = ProcessRequestDto,
    responses(
        (status = 200, description = "Request processed successfully", body = ProcessResponseDto),
        (status = 400, description = "Invalid request", body = ErrorResponseDto),
        (status = 500, description = "Internal server error", body = ErrorResponseDto)
    ),
    tag = "requests"
)]
#[instrument(skip(state, request), fields(query = %request.query))]
pub async fn process_request(
    State(state): State<ApiState>,
    Json(request): Json<ProcessRequestDto>,
) -> Result<Json<ProcessResponseDto>, (StatusCode, Json<ErrorResponseDto>)> {
    info!("Processing MoA request: {}", request.query);

    // Validate request
    if request.query.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponseDto {
                error: "validation_error".to_string(),
                message: "Query cannot be empty".to_string(),
                timestamp: chrono::Utc::now(),
            }),
        ));
    }

    // Process request through MoA engine
    match state.engine.process_query(&request.query, request.context.as_deref()).await {
        Ok(response) => {
            info!("Request processed successfully: {}", response.id);
            Ok(Json(response.into()))
        }
        Err(e) => {
            error!("Failed to process request: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponseDto {
                    error: "processing_error".to_string(),
                    message: format!("Failed to process request: {}", e),
                    timestamp: chrono::Utc::now(),
                }),
            ))
        }
    }
}

/// Get system metrics
#[utoipa::path(
    get,
    path = "/api/v1/metrics",
    responses(
        (status = 200, description = "Metrics retrieved successfully", body = Vec<MetricValueDto>),
        (status = 500, description = "Internal server error", body = ErrorResponseDto)
    ),
    tag = "metrics"
)]
#[instrument(skip(state))]
pub async fn get_metrics(
    State(state): State<ApiState>,
) -> Result<Json<Vec<MetricValueDto>>, (StatusCode, Json<ErrorResponseDto>)> {
    info!("Retrieving system metrics");

    let mut metrics = Vec::new();
    let timestamp = chrono::Utc::now();

    // Get health check for overall system status
    match state.engine.deep_health_check().await {
        Ok(detailed_health) => {
            // Resource metrics
            metrics.push(MetricValueDto {
                name: "resources_status".to_string(),
                value: if detailed_health.resources.status == crate::models::HealthStatus::Healthy {
                    1.0
                } else {
                    0.0
                },
                timestamp,
            });

            // Agent metrics
            let agent_status = detailed_health.agents.status();
            metrics.push(MetricValueDto {
                name: "agents_status".to_string(),
                value: if agent_status == crate::models::HealthStatus::Healthy {
                    1.0
                } else {
                    0.0
                },
                timestamp,
            });
            metrics.push(MetricValueDto {
                name: "agents_total".to_string(),
                value: detailed_health.agents.total_agents as f64,
                timestamp,
            });

            // Strategy metrics
            metrics.push(MetricValueDto {
                name: "strategies_status".to_string(),
                value: if detailed_health.strategies.status == crate::models::HealthStatus::Healthy {
                    1.0
                } else {
                    0.0
                },
                timestamp,
            });

            // Processor metrics
            metrics.push(MetricValueDto {
                name: "processor_status".to_string(),
                value: if detailed_health.processor.status == crate::models::HealthStatus::Healthy {
                    1.0
                } else {
                    0.0
                },
                timestamp,
            });
        }
        Err(e) => {
            error!("Failed to get detailed health for metrics: {}", e);
        }
    }

    // Get agent-specific metrics
    let agents = state.engine.agent_manager().list_agents().await;
    for (agent_id, _config, agent_metrics) in agents {
        metrics.push(MetricValueDto {
            name: format!("agent_{}_success_rate", agent_id),
            value: agent_metrics.success_rate,
            timestamp,
        });
        metrics.push(MetricValueDto {
            name: format!("agent_{}_avg_latency_ms", agent_id),
            value: agent_metrics.avg_latency_ms,
            timestamp,
        });
        metrics.push(MetricValueDto {
            name: format!("agent_{}_total_requests", agent_id),
            value: agent_metrics.total_requests as f64,
            timestamp,
        });
        metrics.push(MetricValueDto {
            name: format!("agent_{}_total_errors", agent_id),
            value: agent_metrics.total_errors as f64,
            timestamp,
        });
    }

    Ok(Json(metrics))
}

/// Get system health status
#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Health check successful", body = HealthStatusDto),
        (status = 503, description = "System unhealthy", body = HealthStatusDto)
    ),
    tag = "health"
)]
#[instrument(skip(state))]
pub async fn get_health(
    State(state): State<ApiState>,
) -> Result<Json<HealthStatusDto>, (StatusCode, Json<HealthStatusDto>)> {
    info!("Performing health check");

    match state.engine.deep_health_check().await {
        Ok(detailed_health) => {
            let status_str = match state.engine.health_check().await {
                Ok(MoaHealthStatus::Healthy) => "healthy".to_string(),
                Ok(MoaHealthStatus::Unhealthy) => "unhealthy".to_string(),
                Ok(MoaHealthStatus::Degraded) => "degraded".to_string(),
                Ok(MoaHealthStatus::Unknown) => "unknown".to_string(),
                Err(_) => "unknown".to_string(),
            };

            let mut components = std::collections::HashMap::new();
            
            // Add component statuses from detailed health
            components.insert("resources".to_string(), ComponentStatusDto {
                name: "resources".to_string(),
                status: format!("{:?}", detailed_health.resources.status),
                message: detailed_health.resources.message.clone(),
                last_check: detailed_health.resources.timestamp,
            });

            components.insert("agents".to_string(), ComponentStatusDto {
                name: "agents".to_string(),
                status: format!("{:?}", detailed_health.agents.status()),
                message: None,
                last_check: detailed_health.agents.timestamp,
            });

            components.insert("strategies".to_string(), ComponentStatusDto {
                name: "strategies".to_string(),
                status: format!("{:?}", detailed_health.strategies.status),
                message: detailed_health.strategies.message.clone(),
                last_check: detailed_health.strategies.timestamp,
            });

            components.insert("processor".to_string(), ComponentStatusDto {
                name: "processor".to_string(),
                status: format!("{:?}", detailed_health.processor.status),
                message: detailed_health.processor.message.clone(),
                last_check: detailed_health.processor.timestamp,
            });

            let health_dto = HealthStatusDto {
                status: status_str.clone(),
                timestamp: detailed_health.timestamp,
                components,
            };

            let status_code = if status_str == "healthy" {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };

            Ok((status_code, Json(health_dto)).1)
        }
        Err(e) => {
            error!("Health check failed: {}", e);
            let health_dto = HealthStatusDto {
                status: "unhealthy".to_string(),
                timestamp: chrono::Utc::now(),
                components: std::collections::HashMap::new(),
            };
            Err((StatusCode::SERVICE_UNAVAILABLE, Json(health_dto)))
        }
    }
}

/// Get agent status
#[utoipa::path(
    get,
    path = "/api/v1/agents/{agent_id}/status",
    responses(
        (status = 200, description = "Agent status retrieved successfully", body = AgentStatusDto),
        (status = 404, description = "Agent not found", body = ErrorResponseDto),
        (status = 500, description = "Internal server error", body = ErrorResponseDto)
    ),
    tag = "agents"
)]
#[instrument(skip(state), fields(agent_id = %agent_id))]
pub async fn get_agent_status(
    State(state): State<ApiState>,
    Path(agent_id): Path<String>,
) -> Result<Json<AgentStatusDto>, (StatusCode, Json<ErrorResponseDto>)> {
    info!("Retrieving agent status: {}", agent_id);

    // Get agent status from engine
    match state.engine.agent_manager().get_agent(&agent_id).await {
        Ok(agent) => {
            let metrics = agent.get_metrics();
            let status_dto = AgentStatusDto {
                agent_id: agent_id.clone(),
                status: "healthy".to_string(),
                metrics: Some(AgentMetricsDto {
                    success_rate: metrics.success_rate,
                    avg_latency_ms: metrics.avg_latency_ms,
                    total_requests: metrics.total_requests,
                    total_errors: metrics.total_errors,
                }),
                last_check: chrono::Utc::now(),
            };
            Ok(Json(status_dto))
        }
        Err(e) => {
            warn!("Agent not found: {}", agent_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponseDto {
                    error: "agent_not_found".to_string(),
                    message: format!("Agent {} not found: {}", agent_id, e),
                    timestamp: chrono::Utc::now(),
                }),
            ))
        }
    }
}

/// Update agent configuration
#[utoipa::path(
    put,
    path = "/api/v1/agents/{agent_id}/config",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Configuration updated successfully"),
        (status = 400, description = "Invalid configuration", body = ErrorResponseDto),
        (status = 404, description = "Agent not found", body = ErrorResponseDto),
        (status = 500, description = "Internal server error", body = ErrorResponseDto)
    ),
    tag = "agents"
)]
#[instrument(skip(state, config), fields(agent_id = %agent_id))]
pub async fn update_agent_config(
    State(state): State<ApiState>,
    Path(agent_id): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponseDto>)> {
    info!("Updating agent configuration: {}", agent_id);

    // Validate agent exists
    if state.engine.agent_manager().get_agent(&agent_id).await.is_err() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponseDto {
                error: "agent_not_found".to_string(),
                message: format!("Agent {} not found", agent_id),
                timestamp: chrono::Utc::now(),
            }),
        ));
    }

    // TODO: Implement actual configuration update
    // For now, just return success
    warn!("Agent configuration update not yet fully implemented");
    Ok(StatusCode::OK)
}

/// Create the API router
pub fn create_router(engine: Arc<MoaEngine>) -> Router {
    let state = ApiState { engine };
    
    Router::new()
        .route("/api/v1/process", post(process_request))
        .route("/api/v1/metrics", get(get_metrics))
        .route("/api/v1/health", get(get_health))
        .route("/api/v1/agents/:agent_id/status", get(get_agent_status))
        .route("/api/v1/agents/:agent_id/config", put(update_agent_config))
        .with_state(state)
}

#[derive(OpenApi)]
#[openapi(
    paths(
        process_request,
        get_metrics,
        get_health,
        get_agent_status,
        update_agent_config,
    ),
    components(
        schemas(ProcessRequestDto, ProcessResponseDto, AgentResponseDto, ResponseMetricsDto, HealthStatusDto, ComponentStatusDto, MetricValueDto, AgentStatusDto, AgentMetricsDto, ErrorResponseDto)
    ),
    tags(
        (name = "requests", description = "Request processing endpoints"),
        (name = "metrics", description = "Metrics and monitoring endpoints"),
        (name = "health", description = "Health check endpoints"),
        (name = "agents", description = "Agent management endpoints")
    )
)]
pub struct ApiDoc;

pub fn api_doc() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
} 