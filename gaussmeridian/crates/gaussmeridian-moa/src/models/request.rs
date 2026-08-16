use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoaRequest {
    pub id: Uuid,
    pub query: String,
    pub context: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl MoaRequest {
    pub fn new(query: String, context: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            query,
            context,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_context(query: String, context: String) -> Self {
        Self::new(query, Some(context))
    }
    
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}