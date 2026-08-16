//! Basic load balancer implementations

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

#[async_trait]
pub trait LoadBalancer: Send + Sync + 'static {
    async fn select_provider(&self, providers: &[String]) -> Option<String>;
}

pub struct RoundRobinLoadBalancer {
    counter: AtomicUsize,
}

impl RoundRobinLoadBalancer {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl LoadBalancer for RoundRobinLoadBalancer {
    async fn select_provider(&self, providers: &[String]) -> Option<String> {
        if providers.is_empty() {
            return None;
        }

        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % providers.len();
        Some(providers[idx].clone())
    }
}
