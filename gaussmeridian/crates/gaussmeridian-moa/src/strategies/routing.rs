use crate::{
    error::{MoaResult, MoaError},
    models::{AgentResponse, MoaRequest, MoaResponse, ResponseMetrics},
    strategies::Strategy,
};
use async_trait::async_trait;

#[derive(Default)]
pub struct RoutingStrategy;

#[async_trait]
impl Strategy for RoutingStrategy {
    async fn process_responses(&self, responses: Vec<AgentResponse>, _request: &MoaRequest) -> MoaResult<MoaResponse> {
        if responses.is_empty() {
            return Err(MoaError::Strategy {
                message: "No responses to process for routing strategy".to_string(),
                source: None
            });
        }

        // Find response with highest confidence
        let best_response = responses.into_iter()
            .max_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap())
            .ok_or_else(|| MoaError::Strategy {
                message: "No response met the scoring threshold for routing strategy".to_string(),
                source: None
            })?;

        Ok(MoaResponse {
            id: uuid::Uuid::new_v4(),
            content: best_response.content.clone(),
            confidence: best_response.confidence as f32,
            agent_responses: vec![best_response],
            timestamp: chrono::Utc::now(),
            metrics: ResponseMetrics::default(),
        })
    }

    async fn warmup(&self) -> MoaResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> MoaResult<()> {
        Ok(())
    }
}