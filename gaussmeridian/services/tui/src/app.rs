//! Application state and main loop for GaussMeridian TUI
//!
//! Handles the main event loop, state management, and coordination
//! between UI components and API client.

use crate::api::ApiClient;
use crate::events::{AppEvent, EventHandler};
use crate::state::{AppState, InputMode, NotificationLevel, View};
use crate::ui;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::interval;

/// Main application struct
pub struct App {
    state: AppState,
    should_quit: bool,
    event_handler: EventHandler,
    api_client: Arc<TokioMutex<ApiClient>>,
}

impl App {
    /// Create a new application instance
    pub fn new(api_base_url: Option<String>, api_key: Option<String>) -> Result<Self, anyhow::Error> {
        let state = AppState::new(api_base_url.clone(), api_key.clone());
        let api_client = ApiClient::new(
            state.api_base_url.clone(),
            state.api_key.clone(),
        )?;
        let event_handler = EventHandler::new(Duration::from_millis(100));

        Ok(Self {
            state,
            should_quit: false,
            event_handler,
            api_client: Arc::new(TokioMutex::new(api_client)),
        })
    }

    /// Run the main application loop
    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        // Initial data load
        self.refresh_data().await;

        // Start auto-refresh task
        let state_clone = self.state.clone();
        let api_client = self.api_client.clone();
        let refresh_interval = self.state.refresh_interval;
        let auto_refresh = self.state.auto_refresh;
        
        tokio::spawn(async move {
            let mut refresh_timer = interval(Duration::from_secs(refresh_interval));
            
            loop {
                refresh_timer.tick().await;
                
                if auto_refresh {
                    let client = api_client.lock().await;
                    Self::refresh_data_internal(&state_clone, &client).await;
                }
            }
        });

        // Main event loop
        loop {
            // Draw UI
            terminal.draw(|f| ui::draw(f, &self.state))?;

            if self.should_quit {
                break;
            }

            // Handle events
            match self.event_handler.next().await {
                AppEvent::Key(key) => {
                    self.handle_key_event(key).await?;
                }
                AppEvent::Mouse(mouse) => {
                    self.handle_mouse_event(mouse).await?;
                }
                AppEvent::Tick => {
                    self.cleanup_notifications();
                }
                AppEvent::Refresh => {
                    self.refresh_data().await;
                }
                AppEvent::Quit => {
                    self.should_quit = true;
                }
                AppEvent::Error(err) => {
                    self.state.set_error(err);
                }
            }
        }

        Ok(())
    }

    /// Handle keyboard events
    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Clear transient messages on key press
        self.state.clear_error();
        self.state.clear_success();

        // Handle input modes first
        match self.state.input_mode {
            InputMode::Search => {
                return self.handle_search_input(key).await;
            }
            InputMode::Editing => {
                return self.handle_edit_input(key).await;
            }
            InputMode::Command => {
                return self.handle_command_input(key).await;
            }
            InputMode::Normal => {}
        }

        // Normal mode key handling
        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                if key.modifiers == KeyModifiers::NONE {
                    self.should_quit = true;
                }
            }
            
            // Refresh
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if key.modifiers == KeyModifiers::NONE {
                    self.refresh_data().await;
                    self.state.add_notification("Data refreshed".to_string(), NotificationLevel::Info);
                }
            }
            
            // Help toggle
            KeyCode::Char('?') => {
                self.state.show_help = !self.state.show_help;
                if self.state.show_help {
                    self.state.navigate_to(View::Help);
                } else if self.state.current_view == View::Help {
                    self.state.go_back();
                }
            }
            
            // Search mode
            KeyCode::Char('/') => {
                self.state.start_search();
            }
            
            // Command mode
            KeyCode::Char(':') => {
                self.state.start_command();
            }
            
            // Escape - close help or cancel
            KeyCode::Esc => {
                if self.state.show_help {
                    self.state.show_help = false;
                    if self.state.current_view == View::Help {
                        self.state.go_back();
                    }
                } else {
                    self.state.cancel_input();
                }
            }
            
            // Direct view switching (number keys)
            KeyCode::Char('1') => self.state.navigate_to(View::Dashboard),
            KeyCode::Char('2') => self.state.navigate_to(View::Provider),
            KeyCode::Char('3') => self.state.navigate_to(View::Model),
            KeyCode::Char('4') => self.state.navigate_to(View::RequestMonitor),
            KeyCode::Char('5') => self.state.navigate_to(View::Agent),
            KeyCode::Char('6') => self.state.navigate_to(View::LogViewer),
            KeyCode::Char('7') => self.state.navigate_to(View::Settings),
            
            // Tab navigation
            KeyCode::Tab => {
                self.cycle_view_forward();
            }
            KeyCode::BackTab => {
                self.cycle_view_backward();
            }
            
            // Focus cycling within view
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.cycle_focus();
            }
            
            // View-specific navigation and actions
            _ => {
                self.handle_view_specific_key(key).await?;
            }
        }

        Ok(())
    }

    /// Handle search input mode
    async fn handle_search_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.state.cancel_input();
            }
            KeyCode::Enter => {
                // Apply search
                let query = self.state.search_query.lock().unwrap().clone();
                match self.state.current_view {
                    View::LogViewer => {
                        *self.state.log_filter.lock().unwrap() = query;
                    }
                    View::RequestMonitor => {
                        *self.state.request_filter.lock().unwrap() = query;
                    }
                    _ => {}
                }
                self.state.cancel_input();
            }
            KeyCode::Char(c) => {
                self.state.search_query.lock().unwrap().push(c);
            }
            KeyCode::Backspace => {
                self.state.search_query.lock().unwrap().pop();
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle edit input mode
    async fn handle_edit_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.state.cancel_input();
            }
            KeyCode::Enter => {
                // Save changes
                let _value = self.state.input_buffer.lock().unwrap().clone();
                // TODO: Apply changes via API
                self.state.add_notification("Changes saved".to_string(), NotificationLevel::Success);
                self.state.cancel_input();
            }
            KeyCode::Char(c) => {
                self.state.input_buffer.lock().unwrap().push(c);
            }
            KeyCode::Backspace => {
                self.state.input_buffer.lock().unwrap().pop();
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle command input mode
    async fn handle_command_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.state.cancel_input();
            }
            KeyCode::Enter => {
                let command = self.state.command_buffer.lock().unwrap().clone();
                self.execute_command(&command).await?;
                self.state.cancel_input();
            }
            KeyCode::Char(c) => {
                self.state.command_buffer.lock().unwrap().push(c);
            }
            KeyCode::Backspace => {
                self.state.command_buffer.lock().unwrap().pop();
            }
            _ => {}
        }
        Ok(())
    }

    /// Execute a command
    async fn execute_command(&mut self, command: &str) -> Result<()> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0].to_lowercase().as_str() {
            "q" | "quit" => {
                self.should_quit = true;
            }
            "refresh" | "r" => {
                self.refresh_data().await;
            }
            "help" | "h" => {
                self.state.navigate_to(View::Help);
            }
            "dashboard" | "dash" => {
                self.state.navigate_to(View::Dashboard);
            }
            "providers" | "p" => {
                self.state.navigate_to(View::Provider);
            }
            "models" | "m" => {
                self.state.navigate_to(View::Model);
            }
            "requests" | "req" => {
                self.state.navigate_to(View::RequestMonitor);
            }
            "agents" | "a" => {
                self.state.navigate_to(View::Agent);
            }
            "logs" | "l" => {
                self.state.navigate_to(View::LogViewer);
            }
            "settings" | "s" => {
                self.state.navigate_to(View::Settings);
            }
            _ => {
                self.state.add_notification(
                    format!("Unknown command: {}", command),
                    NotificationLevel::Warning,
                );
            }
        }
        Ok(())
    }

    /// Handle mouse events
    async fn handle_mouse_event(&mut self, _mouse: crossterm::event::MouseEvent) -> Result<()> {
        // Mouse handling can be implemented here for click navigation
        Ok(())
    }

    /// Cycle to next view
    fn cycle_view_forward(&mut self) {
        let views = [
            View::Dashboard,
            View::Provider,
            View::Model,
            View::RequestMonitor,
            View::Agent,
            View::LogViewer,
            View::Settings,
        ];
        
        let current_idx = views.iter()
            .position(|&v| v == self.state.current_view)
            .unwrap_or(0);
        
        let next_idx = (current_idx + 1) % views.len();
        self.state.navigate_to(views[next_idx]);
    }

    /// Cycle to previous view
    fn cycle_view_backward(&mut self) {
        let views = [
            View::Dashboard,
            View::Provider,
            View::Model,
            View::RequestMonitor,
            View::Agent,
            View::LogViewer,
            View::Settings,
        ];
        
        let current_idx = views.iter()
            .position(|&v| v == self.state.current_view)
            .unwrap_or(0);
        
        let prev_idx = if current_idx == 0 {
            views.len() - 1
        } else {
            current_idx - 1
        };
        self.state.navigate_to(views[prev_idx]);
    }

    /// Handle view-specific key events
    async fn handle_view_specific_key(&mut self, key: KeyEvent) -> Result<()> {
        let item_count = self.state.current_item_count();

        match key.code {
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_up(item_count);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_down(item_count);
            }
            KeyCode::PageUp => {
                self.state.page_up(10);
            }
            KeyCode::PageDown => {
                self.state.page_down(item_count, 10);
            }
            KeyCode::Home => {
                self.state.selected_index = 0;
                self.state.scroll_position = 0;
            }
            KeyCode::End => {
                if item_count > 0 {
                    self.state.selected_index = item_count - 1;
                }
            }
            
            // Actions
            KeyCode::Enter => {
                self.handle_select_action().await?;
            }
            KeyCode::Char('e') => {
                self.handle_enable_toggle().await?;
            }
            KeyCode::Char('d') => {
                // Toggle detail view
                self.state.cycle_focus();
            }
            
            _ => {}
        }

        Ok(())
    }

    /// Handle select action for current item
    async fn handle_select_action(&mut self) -> Result<()> {
        match self.state.current_view {
            View::Provider => {
                // Could open provider details or toggle enable
                self.handle_enable_toggle().await?;
            }
            View::Model => {
                // Show model details
                let models = self.state.models.lock().unwrap();
                if let Some(model) = models.get(self.state.selected_index) {
                    self.state.add_notification(
                        format!("Model: {} ({})", model.name, model.provider),
                        NotificationLevel::Info,
                    );
                }
            }
            View::Agent => {
                // Show agent details
                let agents = self.state.agents.lock().unwrap();
                if let Some(agent) = agents.get(self.state.selected_index) {
                    self.state.add_notification(
                        format!("Agent: {} [{}]", agent.name, agent.strategy),
                        NotificationLevel::Info,
                    );
                }
            }
            View::Settings => {
                // Toggle section expand or start editing
                let mut sections = self.state.config_sections.lock().unwrap();
                if let Some(section) = sections.get_mut(self.state.selected_index) {
                    section.expanded = !section.expanded;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle enable/disable toggle for providers
    async fn handle_enable_toggle(&mut self) -> Result<()> {
        if self.state.current_view != View::Provider {
            return Ok(());
        }

        let mut providers = self.state.providers.lock().unwrap();
        if let Some(provider) = providers.get_mut(self.state.selected_index) {
            provider.enabled = !provider.enabled;
            let status = if provider.enabled { "enabled" } else { "disabled" };
            self.state.add_notification(
                format!("Provider {} {}", provider.name, status),
                NotificationLevel::Success,
            );
            // TODO: Call API to persist change
        }
        Ok(())
    }

    /// Cleanup expired notifications
    fn cleanup_notifications(&self) {
        let now = chrono::Utc::now();
        let mut notifications = self.state.notifications.lock().unwrap();
        notifications.retain(|n| {
            let elapsed = (now - n.timestamp).num_seconds() as u64;
            elapsed < n.duration_secs
        });
    }

    /// Refresh all data from API
    async fn refresh_data(&mut self) {
        let client = self.api_client.lock().await;
        Self::refresh_data_internal(&self.state, &client).await;
    }

    /// Internal data refresh function
    async fn refresh_data_internal(state: &AppState, api_client: &ApiClient) {
        // Fetch data in parallel
        let (health_result, models_result, providers_result, agents_result, requests_result) = tokio::join!(
            api_client.get_health(),
            api_client.get_models(),
            api_client.get_providers(),
            api_client.get_agents(),
            api_client.get_recent_requests(100),
        );

        // Update connection status
        let connected = health_result.is_ok();
        state.set_connected(connected);

        // Update metrics
        if let Ok(metrics) = health_result {
            let mut current_metrics = state.metrics.lock().unwrap();
            
            // Add samples to history
            current_metrics.add_rps_sample(metrics.requests_per_second);
            current_metrics.add_latency_sample(metrics.avg_latency_ms);
            current_metrics.add_error_sample(metrics.error_rate);
            current_metrics.add_memory_sample(metrics.memory_usage_mb);
            
            // Update current values
            current_metrics.uptime_seconds = metrics.uptime_seconds;
            current_metrics.total_requests = metrics.total_requests;
            current_metrics.requests_per_second = metrics.requests_per_second;
            current_metrics.avg_latency_ms = metrics.avg_latency_ms;
            current_metrics.error_rate = metrics.error_rate;
            current_metrics.memory_usage_mb = metrics.memory_usage_mb;
            current_metrics.cpu_usage_percent = metrics.cpu_usage_percent;
            current_metrics.cache_hit_rate = metrics.cache_hit_rate;
        }

        // Update models
        if let Ok(models) = models_result {
            *state.models.lock().unwrap() = models;
        }

        // Update providers
        if let Ok(providers) = providers_result {
            *state.providers.lock().unwrap() = providers;
        }

        // Update agents
        if let Ok(agents) = agents_result {
            *state.agents.lock().unwrap() = agents;
        }

        // Update recent requests
        if let Ok(requests) = requests_result {
            let mut recent = state.recent_requests.lock().unwrap();
            recent.clear();
            recent.extend(requests);
        }

        state.update_refresh_time();
    }
}


