//! Load balancer implementations for provider selection

use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::provider_registry::ProviderEntry;

#[async_trait]
pub trait AdvancedLoadBalancer: Send + Sync + 'static {
    async fn select_provider(&self, providers: &[Arc<ProviderEntry>])
        -> Option<Arc<ProviderEntry>>;
}

pub struct WeightedLoadBalancer {
    providers: Arc<DashMap<String, f64>>,
    counter: AtomicUsize,
}

impl WeightedLoadBalancer {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(DashMap::new()),
            counter: AtomicUsize::new(0),
        }
    }
    pub fn set_weight(&self, provider: String, weight: f64) {
        self.providers.insert(provider, weight);
    }
}

#[async_trait]
impl AdvancedLoadBalancer for WeightedLoadBalancer {
    async fn select_provider(
        &self,
        providers: &[Arc<ProviderEntry>],
    ) -> Option<Arc<ProviderEntry>> {
        if providers.is_empty() {
            return None;
        }
        // Simple round-robin for now; can be replaced with weighted logic
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % providers.len();
        Some(providers[idx].clone())
    }
}

pub struct LeastConnectionsLoadBalancer;

#[async_trait]
impl AdvancedLoadBalancer for LeastConnectionsLoadBalancer {
    async fn select_provider(
        &self,
        providers: &[Arc<ProviderEntry>],
    ) -> Option<Arc<ProviderEntry>> {
        providers
            .iter()
            .min_by_key(|entry| entry.failure_count.load(Ordering::Relaxed))
            .map(|e| e.clone())
    }
}
