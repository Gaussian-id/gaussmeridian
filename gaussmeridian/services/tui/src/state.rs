//! Application state management for GaussMeridian TUI
//!
//! Provides comprehensive state management with thread-safe access
//! to metrics, providers, models, and configuration data.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};

/// Application view types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Provider,
    Model,
    RequestMonitor,
    Agent,
    LogViewer,
    Settings,
    Help,
    Tenants,
}

/// Focus area within views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    Navigation,
    Content,
    Details,
    Input,
}

/// Provider status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub name: String,
    pub enabled: bool,
    pub healthy: bool,
    pub base_url: String,
    pub models: Vec<String>,
    pub last_health_check: Option<DateTime<Utc>>,
    pub request_count: u64,
    pub error_count: u64,
    pub avg_latency_ms: f64,
    pub priority: u32,
    pub weight: f64,
}

impl Default for ProviderStatus {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            healthy: true,
            base_url: String::new(),
            models: Vec::new(),
            last_health_check: None,
            request_count: 0,
            error_count: 0,
            avg_latency_ms: 0.0,
            priority: 0,
            weight: 1.0,
        }
    }
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub enabled: bool,
    pub context_length: Option<u32>,
    pub pricing: Option<ModelPricing>,
    pub request_count: u64,
    pub avg_latency_ms: f64,
    pub capabilities: Vec<String>,
}

impl Default for ModelInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            provider: String::new(),
            enabled: true,
            context_length: None,
            pricing: None,
            request_count: 0,
            avg_latency_ms: 0.0,
            capabilities: Vec::new(),
        }
    }
}

/// Model pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub prompt_cost_per_1k: f64,
    pub completion_cost_per_1k: f64,
    pub currency: String,
}

/// Request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInfo {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub endpoint: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub status_code: u16,
    pub latency_ms: f64,
    pub tokens: Option<u32>,
    pub cost: Option<f64>,
    pub error: Option<String>,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
}

/// Agent status information (MoA)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub strategy: String,
    pub status: String,
    pub request_count: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub last_activity: Option<DateTime<Utc>>,
    pub config: HashMap<String, String>,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            agent_type: "llm".to_string(),
            strategy: "standard".to_string(),
            status: "idle".to_string(),
            request_count: 0,
            success_rate: 1.0,
            avg_latency_ms: 0.0,
            last_activity: None,
            config: HashMap::new(),
        }
    }
}

/// System metrics with historical data for sparklines
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub requests_per_second: f64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub error_rate: f64,
    pub memory_usage_mb: f64,
    pub memory_total_mb: f64,
    pub cpu_usage_percent: f64,
    pub cache_hit_rate: f64,
    pub active_connections: u32,
    pub total_tokens_processed: u64,
    pub total_cost: f64,
    // Historical data for sparklines (last 60 data points)
    pub rps_history: VecDeque<f64>,
    pub latency_history: VecDeque<f64>,
    pub error_history: VecDeque<f64>,
    pub memory_history: VecDeque<f64>,
}

impl SystemMetrics {
    pub fn add_rps_sample(&mut self, rps: f64) {
        if self.rps_history.len() >= 60 {
            self.rps_history.pop_front();
        }
        self.rps_history.push_back(rps);
    }

    pub fn add_latency_sample(&mut self, latency: f64) {
        if self.latency_history.len() >= 60 {
            self.latency_history.pop_front();
        }
        self.latency_history.push_back(latency);
    }

    pub fn add_error_sample(&mut self, error_rate: f64) {
        if self.error_history.len() >= 60 {
            self.error_history.pop_front();
        }
        self.error_history.push_back(error_rate);
    }

    pub fn add_memory_sample(&mut self, memory: f64) {
        if self.memory_history.len() >= 60 {
            self.memory_history.pop_front();
        }
        self.memory_history.push_back(memory);
    }
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub target: String,
    pub span: Option<String>,
}

/// Tenant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantInfo {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub balance: f64,
    pub currency: String,
    pub api_key_count: u32,
    pub request_count: u64,
    pub created_at: DateTime<Utc>,
}

/// Configuration section for settings view
#[derive(Debug, Clone)]
pub struct ConfigSection {
    pub name: String,
    pub items: Vec<ConfigItem>,
    pub expanded: bool,
}

/// Configuration item
#[derive(Debug, Clone)]
pub struct ConfigItem {
    pub key: String,
    pub value: String,
    pub editable: bool,
    pub description: String,
}

/// Input mode for forms
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
    Search,
    Command,
}

/// Application state
#[derive(Clone)]
pub struct AppState {
    // View state
    pub current_view: View,
    pub previous_view: Option<View>,
    pub focus_area: FocusArea,
    pub selected_index: usize,
    pub scroll_position: usize,
    pub detail_scroll: usize,

    // Data collections
    pub providers: Arc<Mutex<Vec<ProviderStatus>>>,
    pub models: Arc<Mutex<Vec<ModelInfo>>>,
    pub recent_requests: Arc<Mutex<VecDeque<RequestInfo>>>,
    pub agents: Arc<Mutex<Vec<AgentStatus>>>,
    pub tenants: Arc<Mutex<Vec<TenantInfo>>>,
    pub metrics: Arc<Mutex<SystemMetrics>>,
    pub logs: Arc<Mutex<VecDeque<LogEntry>>>,
    pub config_sections: Arc<Mutex<Vec<ConfigSection>>>,

    // Input state
    pub input_mode: InputMode,
    pub input_buffer: Arc<Mutex<String>>,
    pub search_query: Arc<Mutex<String>>,
    pub command_buffer: Arc<Mutex<String>>,

    // Filters
    pub log_filter: Arc<Mutex<String>>,
    pub log_level_filter: Arc<Mutex<Option<String>>>,
    pub request_filter: Arc<Mutex<String>>,

    // Messages and errors
    pub error_message: Arc<Mutex<Option<String>>>,
    pub success_message: Arc<Mutex<Option<String>>>,
    pub notifications: Arc<Mutex<VecDeque<Notification>>>,

    // Connection state
    pub api_base_url: String,
    pub api_key: Option<String>,
    pub connected: Arc<RwLock<bool>>,
    pub last_refresh: Arc<RwLock<Option<DateTime<Utc>>>>,

    // Settings
    pub auto_refresh: bool,
    pub refresh_interval: u64,
    pub show_help: bool,
    pub compact_mode: bool,
    pub dark_mode: bool,
}

/// Notification for toast-style messages
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub timestamp: DateTime<Utc>,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_view: View::Dashboard,
            previous_view: None,
            focus_area: FocusArea::Navigation,
            selected_index: 0,
            scroll_position: 0,
            detail_scroll: 0,

            providers: Arc::new(Mutex::new(Vec::new())),
            models: Arc::new(Mutex::new(Vec::new())),
            recent_requests: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            agents: Arc::new(Mutex::new(Vec::new())),
            tenants: Arc::new(Mutex::new(Vec::new())),
            metrics: Arc::new(Mutex::new(SystemMetrics::default())),
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(5000))),
            config_sections: Arc::new(Mutex::new(Vec::new())),

            input_mode: InputMode::Normal,
            input_buffer: Arc::new(Mutex::new(String::new())),
            search_query: Arc::new(Mutex::new(String::new())),
            command_buffer: Arc::new(Mutex::new(String::new())),

            log_filter: Arc::new(Mutex::new(String::new())),
            log_level_filter: Arc::new(Mutex::new(None)),
            request_filter: Arc::new(Mutex::new(String::new())),

            error_message: Arc::new(Mutex::new(None)),
            success_message: Arc::new(Mutex::new(None)),
            notifications: Arc::new(Mutex::new(VecDeque::with_capacity(10))),

            api_base_url: std::env::var("GAUSSMERIDIAN_API_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
            api_key: std::env::var("GAUSSMERIDIAN_API_KEY").ok(),
            connected: Arc::new(RwLock::new(false)),
            last_refresh: Arc::new(RwLock::new(None)),

            auto_refresh: true,
            refresh_interval: 3,
            show_help: false,
            compact_mode: false,
            dark_mode: true,
        }
    }
}

impl AppState {
    /// Create a new application state
    pub fn new(api_base_url: Option<String>, api_key: Option<String>) -> Self {
        let mut state = Self::default();
        if let Some(url) = api_base_url {
            state.api_base_url = url;
        }
        if api_key.is_some() {
            state.api_key = api_key;
        }
        state.init_config_sections();
        state
    }

    /// Initialize configuration sections
    fn init_config_sections(&self) {
        let sections = vec![
            ConfigSection {
                name: "Server".to_string(),
                items: vec![
                    ConfigItem {
                        key: "host".to_string(),
                        value: "0.0.0.0".to_string(),
                        editable: false,
                        description: "Server bind address".to_string(),
                    },
                    ConfigItem {
                        key: "port".to_string(),
                        value: "8000".to_string(),
                        editable: false,
                        description: "Server port".to_string(),
                    },
                ],
                expanded: true,
            },
            ConfigSection {
                name: "Cache".to_string(),
                items: vec![
                    ConfigItem {
                        key: "enabled".to_string(),
                        value: "true".to_string(),
                        editable: true,
                        description: "Enable response caching".to_string(),
                    },
                    ConfigItem {
                        key: "ttl_seconds".to_string(),
                        value: "3600".to_string(),
                        editable: true,
                        description: "Cache TTL in seconds".to_string(),
                    },
                ],
                expanded: false,
            },
            ConfigSection {
                name: "Rate Limiting".to_string(),
                items: vec![
                    ConfigItem {
                        key: "enabled".to_string(),
                        value: "true".to_string(),
                        editable: true,
                        description: "Enable rate limiting".to_string(),
                    },
                    ConfigItem {
                        key: "requests_per_minute".to_string(),
                        value: "60".to_string(),
                        editable: true,
                        description: "Default requests per minute".to_string(),
                    },
                ],
                expanded: false,
            },
        ];
        *self.config_sections.lock().unwrap() = sections;
    }

    /// Navigate to a different view
    pub fn navigate_to(&mut self, view: View) {
        if self.current_view != view {
            self.previous_view = Some(self.current_view);
            self.current_view = view;
            self.selected_index = 0;
            self.scroll_position = 0;
            self.detail_scroll = 0;
            self.focus_area = FocusArea::Content;
        }
    }

    /// Go back to previous view
    pub fn go_back(&mut self) {
        if let Some(prev) = self.previous_view.take() {
            self.current_view = prev;
            self.selected_index = 0;
            self.scroll_position = 0;
        }
    }

    /// Cycle focus area
    pub fn cycle_focus(&mut self) {
        self.focus_area = match self.focus_area {
            FocusArea::Navigation => FocusArea::Content,
            FocusArea::Content => FocusArea::Details,
            FocusArea::Details => FocusArea::Navigation,
            FocusArea::Input => FocusArea::Content,
        };
    }

    /// Move selection up
    pub fn select_up(&mut self, max_items: usize) {
        if max_items == 0 {
            return;
        }
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = max_items.saturating_sub(1);
        }
        self.ensure_visible(max_items, 15);
    }

    /// Move selection down
    pub fn select_down(&mut self, max_items: usize) {
        if max_items == 0 {
            return;
        }
        if self.selected_index < max_items.saturating_sub(1) {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
        self.ensure_visible(max_items, 15);
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self, _max_items: usize, visible_height: usize) {
        if self.selected_index < self.scroll_position {
            self.scroll_position = self.selected_index;
        } else if self.selected_index >= self.scroll_position + visible_height {
            self.scroll_position = self.selected_index.saturating_sub(visible_height) + 1;
        }
    }

    /// Scroll up by page
    pub fn page_up(&mut self, page_size: usize) {
        self.scroll_position = self.scroll_position.saturating_sub(page_size);
        if self.selected_index > self.scroll_position + page_size {
            self.selected_index = self.scroll_position;
        }
    }

    /// Scroll down by page
    pub fn page_down(&mut self, max_items: usize, page_size: usize) {
        let max_scroll = max_items.saturating_sub(page_size);
        self.scroll_position = (self.scroll_position + page_size).min(max_scroll);
        if self.selected_index < self.scroll_position {
            self.selected_index = self.scroll_position;
        }
    }

    /// Set error message
    pub fn set_error(&self, error: String) {
        *self.error_message.lock().unwrap() = Some(error.clone());
        self.add_notification(error, NotificationLevel::Error);
    }

    /// Set success message
    pub fn set_success(&self, message: String) {
        *self.success_message.lock().unwrap() = Some(message.clone());
        self.add_notification(message, NotificationLevel::Success);
    }

    /// Add notification
    pub fn add_notification(&self, message: String, level: NotificationLevel) {
        let mut notifications = self.notifications.lock().unwrap();
        if notifications.len() >= 10 {
            notifications.pop_front();
        }
        notifications.push_back(Notification {
            message,
            level,
            timestamp: Utc::now(),
            duration_secs: 5,
        });
    }

    /// Clear error message
    pub fn clear_error(&self) {
        *self.error_message.lock().unwrap() = None;
    }

    /// Clear success message
    pub fn clear_success(&self) {
        *self.success_message.lock().unwrap() = None;
    }

    /// Update last refresh time
    pub fn update_refresh_time(&self) {
        *self.last_refresh.write().unwrap() = Some(Utc::now());
    }

    /// Set connection status
    pub fn set_connected(&self, connected: bool) {
        *self.connected.write().unwrap() = connected;
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        *self.connected.read().unwrap()
    }

    /// Start editing mode
    pub fn start_editing(&mut self) {
        self.input_mode = InputMode::Editing;
        self.focus_area = FocusArea::Input;
    }

    /// Start search mode
    pub fn start_search(&mut self) {
        self.input_mode = InputMode::Search;
        self.focus_area = FocusArea::Input;
        *self.search_query.lock().unwrap() = String::new();
    }

    /// Start command mode
    pub fn start_command(&mut self) {
        self.input_mode = InputMode::Command;
        self.focus_area = FocusArea::Input;
        *self.command_buffer.lock().unwrap() = String::new();
    }

    /// Cancel input mode
    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.focus_area = FocusArea::Content;
        *self.input_buffer.lock().unwrap() = String::new();
        *self.search_query.lock().unwrap() = String::new();
        *self.command_buffer.lock().unwrap() = String::new();
    }

    /// Get view title
    pub fn view_title(&self) -> &'static str {
        match self.current_view {
            View::Dashboard => "Dashboard",
            View::Provider => "Providers",
            View::Model => "Models",
            View::RequestMonitor => "Request Monitor",
            View::Agent => "MoA Agents",
            View::LogViewer => "Logs",
            View::Settings => "Settings",
            View::Tenants => "Tenants",
            View::Help => "Help",
        }
    }

    /// Get item count for current view
    pub fn current_item_count(&self) -> usize {
        match self.current_view {
            View::Dashboard => 0,
            View::Provider => self.providers.lock().unwrap().len(),
            View::Model => self.models.lock().unwrap().len(),
            View::RequestMonitor => self.recent_requests.lock().unwrap().len(),
            View::Agent => self.agents.lock().unwrap().len(),
            View::LogViewer => self.logs.lock().unwrap().len(),
            View::Settings => self.config_sections.lock().unwrap().len(),
            View::Tenants => self.tenants.lock().unwrap().len(),
            View::Help => 0,
        }
    }
}
