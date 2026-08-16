//! Request batching for optimizing throughput

use crate::error::GaussMeridianError;
use crate::provider_registry::ProviderRegistry;
use gaussmeridian_models::{ChatCompletionRequest, ChatCompletionResponse};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, instrument};

/// Request batcher for optimizing throughput
pub struct RequestBatcher {
    tx: mpsc::Sender<BatchRequest>,
    batch_size: usize,
    batch_timeout: Duration,
}

#[derive(Debug)]
pub struct BatchRequest {
    pub id: String,
    pub request: ChatCompletionRequest,
    pub response_tx:
        tokio::sync::oneshot::Sender<Result<ChatCompletionResponse, GaussMeridianError>>,
}

impl RequestBatcher {
    pub fn new(
        batch_size: usize,
        batch_timeout: Duration,
        registry: Arc<ProviderRegistry>,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel(1000);
        let registry_clone = registry.clone();

        tokio::spawn(async move {
            let mut batch = Vec::new();
            let mut timer = tokio::time::interval(batch_timeout);
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    request = rx.recv() => {
                        if let Some(request) = request {
                            batch.push(request);
                            if batch.len() >= batch_size {
                                Self::process_batch(batch, registry_clone.clone()).await;
                                batch = Vec::new();
                                timer.reset();
                            }
                        } else {
                            // Channel closed, process remaining batch
                            if !batch.is_empty() {
                                Self::process_batch(batch, registry_clone.clone()).await;
                            }
                            break;
                        }
                    }
                    _ = timer.tick() => {
                        if !batch.is_empty() {
                            let current_batch = std::mem::take(&mut batch);
                            Self::process_batch(current_batch, registry_clone.clone()).await;
                        }
                    }
                }
            }
        });

        Self {
            tx,
            batch_size,
            batch_timeout,
        }
    }

    #[instrument(skip(registry))]
    async fn process_batch(batch: Vec<BatchRequest>, registry: Arc<ProviderRegistry>) {
        info!("Processing batch of {} requests", batch.len());

        if batch.is_empty() {
            return;
        }

        // Group requests by model for efficient batching
        let mut model_groups: std::collections::HashMap<String, Vec<BatchRequest>> =
            std::collections::HashMap::new();
        for request in batch {
            let model = request.request.model.clone();
            model_groups
                .entry(model)
                .or_insert_with(Vec::new)
                .push(request);
        }

        // Process each model group in parallel
        let mut handles = Vec::new();
        for (model, requests) in model_groups {
            let registry_clone = registry.clone();
            let handle = tokio::spawn(async move {
                Self::process_model_batch(requests, registry_clone, &model).await;
            });
            handles.push(handle);
        }

        // Wait for all batches to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    #[instrument(skip(registry))]
    async fn process_model_batch(
        requests: Vec<BatchRequest>,
        registry: Arc<ProviderRegistry>,
        model: &str,
    ) {
        // Find a provider that supports this model
        let providers = registry.all();
        let mut provider_entry: Option<Arc<crate::provider_registry::ProviderEntry>> = None;

        // Check each provider to find one that supports the model
        for entry in providers {
            if entry.provider.supports_model(model).await {
                provider_entry = Some(entry);
                break;
            }
        }

        if let Some(provider_entry) = provider_entry {
            // Process requests in parallel for this model
            let mut handles = Vec::new();
            for batch_request in requests {
                let provider_clone = provider_entry.provider.clone();
                let request = batch_request.request.clone();
                let response_tx = batch_request.response_tx;
                let request_id = batch_request.id.clone();

                let handle = tokio::spawn(async move {
                    let result = provider_clone.chat_completion(request).await.map_err(|e| {
                        GaussMeridianError::ProviderError(format!("Provider error: {}", e))
                    });

                    if let Err(e) = response_tx.send(result) {
                        error!(
                            "Failed to send batch response for request {}: {:?}",
                            request_id, e
                        );
                    }
                });
                handles.push(handle);
            }

            // Wait for all requests in this batch to complete
            for handle in handles {
                let _ = handle.await;
            }
        } else {
            // No provider found, send errors
            error!("No provider found for model: {}", model);
            for batch_request in requests {
                let _ = batch_request
                    .response_tx
                    .send(Err(GaussMeridianError::ProviderError(format!(
                        "No provider found for model: {}",
                        model
                    ))));
            }
        }
    }

    pub async fn submit(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, GaussMeridianError> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let batch_request = BatchRequest {
            id: uuid::Uuid::new_v4().to_string(),
            request,
            response_tx,
        };

        self.tx
            .send(batch_request)
            .await
            .map_err(|_| GaussMeridianError::ProviderError("Batch channel closed".to_string()))?;

        response_rx
            .await
            .map_err(|_| GaussMeridianError::ProviderError("Response channel closed".to_string()))?
    }
}
