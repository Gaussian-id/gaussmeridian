//! Connection pooling for managing provider connections

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Connection pool for managing provider connections
pub struct ConnectionPool {
    max_connections: usize,
    active_connections: Arc<AtomicUsize>,
}

impl ConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            active_connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn acquire(&self) -> Result<ConnectionGuard, ()> {
        let current = self.active_connections.load(Ordering::Relaxed);
        if current >= self.max_connections {
            return Err(());
        }

        self.active_connections.fetch_add(1, Ordering::Relaxed);
        Ok(ConnectionGuard {
            active_connections: self.active_connections.clone(),
        })
    }

    pub fn active_count(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }
}

pub struct ConnectionGuard {
    active_connections: Arc<AtomicUsize>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
}
