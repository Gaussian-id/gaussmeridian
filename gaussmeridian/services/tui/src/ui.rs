//! Professional UI rendering for GaussMeridian TUI
//!
//! Elegant and modern terminal interface with professional styling,
//! real-time metrics visualization, and comprehensive admin views.

use crate::state::{AppState, FocusArea, InputMode, NotificationLevel, View};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, Paragraph,
        Row, Sparkline, Table, Tabs, Wrap,
    },
};

// ============================================================================
// Theme Configuration
// ============================================================================

/// Professional color theme for the TUI
pub struct Theme {
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_highlight: Color,
    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_muted: Color,
    pub accent: Color,
    pub accent_secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub border: Color,
    pub border_focus: Color,
    pub selection: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg_primary: Color::Rgb(17, 17, 27),
            bg_secondary: Color::Rgb(24, 24, 37),
            bg_highlight: Color::Rgb(49, 50, 68),
            fg_primary: Color::Rgb(205, 214, 244),
            fg_secondary: Color::Rgb(166, 173, 200),
            fg_muted: Color::Rgb(108, 112, 134),
            accent: Color::Rgb(137, 180, 250),
            accent_secondary: Color::Rgb(180, 190, 254),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            info: Color::Rgb(148, 226, 213),
            border: Color::Rgb(69, 71, 90),
            border_focus: Color::Rgb(137, 180, 250),
            selection: Color::Rgb(88, 91, 112),
        }
    }
}

fn theme() -> Theme {
    Theme::dark()
}

// ============================================================================
// Main Draw Function
// ============================================================================

pub fn draw(frame: &mut Frame, state: &AppState) {
    let t = theme();
    
    // Create main layout
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Status bar
        ])
        .split(frame.size());

    // Fill background
    let bg_block = Block::default().style(Style::default().bg(t.bg_primary));
    frame.render_widget(bg_block, frame.size());

    // Render components
    draw_header(frame, main_chunks[0], state, &t);
    draw_content(frame, main_chunks[1], state, &t);
    draw_status_bar(frame, main_chunks[2], state, &t);

    // Draw help overlay if active
    if state.show_help || state.current_view == View::Help {
        draw_help_overlay(frame, state, &t);
    }

    // Draw notifications
    draw_notifications(frame, state, &t);
}

// ============================================================================
// Header Component
// ============================================================================

fn draw_header(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(28), // Logo
            Constraint::Min(0),     // Tabs
            Constraint::Length(20), // Status
        ])
        .split(area);

    // Logo
    let logo = Paragraph::new(Line::from(vec![
        Span::styled("⚡", Style::default().fg(t.accent)),
        Span::styled(" GaussMeridian ", Style::default().fg(t.fg_primary).add_modifier(Modifier::BOLD)),
        Span::styled("TUI", Style::default().fg(t.accent_secondary)),
    ]))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border))
        .style(Style::default().bg(t.bg_secondary)));
    
    frame.render_widget(logo, header_chunks[0]);

    // Navigation tabs
    let tab_titles = vec![
        "󰕮 Dashboard",
        "󰒍 Providers",
        "󰘬 Models",
        "󰑐 Requests",
        "󰯙 Agents",
        "󰌱 Logs",
        "󰒓 Settings",
    ];

    let selected_idx = match state.current_view {
            View::Dashboard => 0,
            View::Provider => 1,
            View::Model => 2,
            View::RequestMonitor => 3,
            View::Agent => 4,
            View::LogViewer => 5,
        View::Settings => 6,
            View::Tenants => 6,
        View::Help => 0,
    };

    let tabs = Tabs::new(tab_titles)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(t.border))
            .style(Style::default().bg(t.bg_secondary)))
        .select(selected_idx)
        .style(Style::default().fg(t.fg_muted))
        .highlight_style(Style::default()
            .fg(t.accent)
            .add_modifier(Modifier::BOLD))
        .divider(Span::styled(" │ ", Style::default().fg(t.border)));

    frame.render_widget(tabs, header_chunks[1]);

    // Connection status
    let connected = state.is_connected();
    let status_text = if connected { "● Online" } else { "○ Offline" };
    let status_color = if connected { t.success } else { t.error };

    let status = Paragraph::new(Line::from(vec![
        Span::styled(status_text, Style::default().fg(status_color)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border))
        .style(Style::default().bg(t.bg_secondary)));

    frame.render_widget(status, header_chunks[2]);
}

// ============================================================================
// Content Router
// ============================================================================

fn draw_content(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    match state.current_view {
        View::Dashboard => draw_dashboard(frame, area, state, t),
        View::Provider => draw_providers(frame, area, state, t),
        View::Model => draw_models(frame, area, state, t),
        View::RequestMonitor => draw_requests(frame, area, state, t),
        View::Agent => draw_agents(frame, area, state, t),
        View::LogViewer => draw_logs(frame, area, state, t),
        View::Settings | View::Tenants => draw_settings(frame, area, state, t),
        View::Help => {} // Help is drawn as overlay
    }
}

// ============================================================================
// Dashboard View
// ============================================================================

fn draw_dashboard(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),  // Metrics cards
            Constraint::Length(8),  // Sparklines
            Constraint::Min(0),     // Tables
        ])
        .margin(1)
        .split(area);

    // Metrics cards row
    draw_metrics_cards(frame, chunks[0], state, t);
    
    // Sparklines row
    draw_sparklines(frame, chunks[1], state, t);
    
    // Bottom tables
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[2]);

    draw_provider_summary(frame, bottom_chunks[0], state, t);
    draw_recent_requests_table(frame, bottom_chunks[1], state, t);
}

fn draw_metrics_cards(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let metrics = state.metrics.lock().unwrap();
    
    let card_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
        ])
        .split(area);

    // Requests/sec card
    draw_metric_card(
        frame, card_chunks[0], t,
        "󰑐 Requests/sec",
        &format!("{:.1}", metrics.requests_per_second),
        t.accent,
        Some((metrics.requests_per_second / 100.0).min(1.0)),
    );

    // Latency card
    let latency_color = if metrics.avg_latency_ms < 50.0 {
        t.success
    } else if metrics.avg_latency_ms < 100.0 {
        t.warning
    } else {
        t.error
    };
    draw_metric_card(
        frame, card_chunks[1], t,
        "󰔛 Avg Latency",
        &format!("{:.1}ms", metrics.avg_latency_ms),
        latency_color,
        Some((1.0 - (metrics.avg_latency_ms / 200.0)).max(0.0)),
    );

    // Error rate card
    let error_color = if metrics.error_rate < 0.01 {
        t.success
    } else if metrics.error_rate < 0.05 {
        t.warning
    } else {
        t.error
    };
    draw_metric_card(
        frame, card_chunks[2], t,
        "󰀦 Error Rate",
        &format!("{:.2}%", metrics.error_rate * 100.0),
        error_color,
        Some(1.0 - metrics.error_rate),
    );

    // Cache hit rate card
    draw_metric_card(
        frame, card_chunks[3], t,
        "󰆼 Cache Hit",
        &format!("{:.1}%", metrics.cache_hit_rate * 100.0),
        t.info,
        Some(metrics.cache_hit_rate),
    );

    // Memory usage card
    let memory_percent = if metrics.memory_total_mb > 0.0 {
        metrics.memory_usage_mb / metrics.memory_total_mb
    } else {
        metrics.memory_usage_mb / 512.0
    };
    let memory_color = if memory_percent < 0.7 {
        t.success
    } else if memory_percent < 0.9 {
        t.warning
    } else {
        t.error
    };
    draw_metric_card(
        frame, card_chunks[4], t,
        "󰍛 Memory",
        &format!("{:.0}MB", metrics.memory_usage_mb),
        memory_color,
        Some(memory_percent.min(1.0)),
    );
}

fn draw_metric_card(
    frame: &mut Frame,
    area: Rect,
    t: &Theme,
    title: &str,
    value: &str,
    color: Color,
    gauge_value: Option<f64>,
) {
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(2), // Value
            Constraint::Min(0),    // Gauge
        ])
        .margin(1)
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border))
        .style(Style::default().bg(t.bg_secondary));
    
    frame.render_widget(block, area);

    // Title
    let title_widget = Paragraph::new(Span::styled(
        title,
        Style::default().fg(t.fg_muted).add_modifier(Modifier::DIM),
    ));
    frame.render_widget(title_widget, inner_chunks[0]);

    // Value
    let value_widget = Paragraph::new(Span::styled(
        value,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(value_widget, inner_chunks[1]);

    // Gauge
    if let Some(ratio) = gauge_value {
        if inner_chunks[2].height >= 1 {
            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(color).bg(t.bg_highlight))
                .ratio(ratio.clamp(0.0, 1.0))
                .label("");
            frame.render_widget(gauge, inner_chunks[2]);
        }
    }
}

fn draw_sparklines(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let metrics = state.metrics.lock().unwrap();
    
    let spark_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // RPS sparkline
    let rps_data: Vec<u64> = metrics.rps_history
        .iter()
        .map(|&v| (v * 10.0) as u64)
        .collect();
    
    let rps_sparkline = Sparkline::default()
        .block(Block::default()
            .title(Span::styled(" 󰄙 Requests/sec (60s) ", Style::default().fg(t.fg_secondary)))
        .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(t.border))
            .style(Style::default().bg(t.bg_secondary)))
        .data(&rps_data)
        .style(Style::default().fg(t.accent));
    
    frame.render_widget(rps_sparkline, spark_chunks[0]);

    // Latency sparkline
    let latency_data: Vec<u64> = metrics.latency_history
        .iter()
        .map(|&v| v as u64)
        .collect();
    
    let latency_sparkline = Sparkline::default()
        .block(Block::default()
            .title(Span::styled(" 󰔛 Latency ms (60s) ", Style::default().fg(t.fg_secondary)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(t.border))
            .style(Style::default().bg(t.bg_secondary)))
        .data(&latency_data)
        .style(Style::default().fg(t.info));
    
    frame.render_widget(latency_sparkline, spark_chunks[1]);
}

fn draw_provider_summary(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let providers = state.providers.lock().unwrap();
    
    let rows: Vec<Row> = providers.iter().map(|p| {
        let status_icon = if p.healthy {
            Span::styled("●", Style::default().fg(t.success))
    } else {
            Span::styled("●", Style::default().fg(t.error))
        };
        
        let enabled_text = if p.enabled {
            Span::styled("ON", Style::default().fg(t.success))
                } else {
            Span::styled("OFF", Style::default().fg(t.fg_muted))
        };

        Row::new(vec![
            Cell::from(status_icon),
            Cell::from(Span::styled(&p.name, Style::default().fg(t.fg_primary))),
            Cell::from(enabled_text),
            Cell::from(Span::styled(
                format!("{}", p.models.len()),
                Style::default().fg(t.fg_secondary),
            )),
            Cell::from(Span::styled(
                format!("{:.0}ms", p.avg_latency_ms),
                Style::default().fg(t.fg_secondary),
            )),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Min(12),
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new(vec!["", "Provider", "St", "Models", "Latency"])
            .style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
    )
    .block(Block::default()
        .title(Span::styled(" 󰒍 Provider Health ", Style::default().fg(t.fg_secondary)))
                .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border))
        .style(Style::default().bg(t.bg_secondary)));

    frame.render_widget(table, area);
}

fn draw_recent_requests_table(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let requests = state.recent_requests.lock().unwrap();
    
    let rows: Vec<Row> = requests.iter().rev().take(10).map(|r| {
            let status_color = if r.status_code < 300 {
            t.success
            } else if r.status_code < 500 {
            t.warning
            } else {
            t.error
            };

            Row::new(vec![
            Cell::from(Span::styled(
                r.timestamp.format("%H:%M:%S").to_string(),
                Style::default().fg(t.fg_muted),
            )),
            Cell::from(Span::styled(
                r.model.clone().unwrap_or_else(|| "-".to_string()),
                Style::default().fg(t.fg_secondary),
            )),
            Cell::from(Span::styled(
                r.status_code.to_string(),
                Style::default().fg(status_color),
            )),
            Cell::from(Span::styled(
                format!("{:.0}ms", r.latency_ms),
                Style::default().fg(t.fg_secondary),
            )),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Min(15),
            Constraint::Length(6),
            Constraint::Length(8),
        ],
    )
        .header(
        Row::new(vec!["Time", "Model", "Code", "Latency"])
            .style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
        )
    .block(Block::default()
        .title(Span::styled(" 󰑐 Recent Requests ", Style::default().fg(t.fg_secondary)))
                .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border))
        .style(Style::default().bg(t.bg_secondary)));

    frame.render_widget(table, area);
}

// ============================================================================
// Providers View
// ============================================================================

fn draw_providers(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .margin(1)
        .split(area);

    // Provider list
    let providers = state.providers.lock().unwrap();
    
    let items: Vec<ListItem> = providers.iter().enumerate().map(|(idx, p)| {
            let is_selected = idx == state.selected_index;
        
        let status_icon = if p.healthy {
            "●"
            } else {
            "○"
        };
        let status_color = if p.healthy { t.success } else { t.error };

            let style = if is_selected {
            Style::default().bg(t.selection)
            } else {
                Style::default()
            };

            ListItem::new(vec![
                Line::from(vec![
                Span::styled(status_icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                Span::styled(&p.name, Style::default().fg(t.fg_primary).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                if p.enabled {
                    Span::styled("[ON]", Style::default().fg(t.success))
                } else {
                    Span::styled("[OFF]", Style::default().fg(t.fg_muted))
                },
                ]),
                Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&p.base_url, Style::default().fg(t.fg_muted).add_modifier(Modifier::DIM)),
                ]),
                Line::from(vec![
                Span::styled("  Models: ", Style::default().fg(t.fg_muted)),
                Span::styled(p.models.len().to_string(), Style::default().fg(t.accent)),
                Span::styled(" │ Requests: ", Style::default().fg(t.fg_muted)),
                Span::styled(p.request_count.to_string(), Style::default().fg(t.info)),
                Span::styled(" │ Errors: ", Style::default().fg(t.fg_muted)),
                    Span::styled(
                        p.error_count.to_string(),
                    Style::default().fg(if p.error_count > 0 { t.error } else { t.success }),
                    ),
                ]),
        ]).style(style)
    }).collect();

    let list_block = Block::default()
        .title(Span::styled(
            format!(" 󰒍 Providers ({}) ", providers.len()),
            Style::default().fg(t.fg_secondary),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if state.focus_area == FocusArea::Content { t.border_focus } else { t.border }
        ))
        .style(Style::default().bg(t.bg_secondary));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(Style::default().bg(t.selection));

    frame.render_widget(list, chunks[0]);

    // Provider details panel
    if let Some(provider) = providers.get(state.selected_index) {
        draw_provider_details(frame, chunks[1], provider, t);
    } else {
        let empty = Paragraph::new("Select a provider to view details")
            .alignment(Alignment::Center)
            .block(Block::default()
                .title(" 󰋽 Details ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(t.border))
                .style(Style::default().bg(t.bg_secondary)));
        frame.render_widget(empty, chunks[1]);
    }
}

fn draw_provider_details(frame: &mut Frame, area: Rect, provider: &crate::state::ProviderStatus, t: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Info
            Constraint::Min(0),     // Models list
        ])
        .margin(1)
        .split(area);

    let block = Block::default()
        .title(Span::styled(
            format!(" 󰋽 {} Details ", provider.name),
            Style::default().fg(t.fg_secondary),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border))
        .style(Style::default().bg(t.bg_secondary));
    
    frame.render_widget(block, area);

    // Provider info
    let info_text = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(t.fg_muted)),
            if provider.healthy {
                Span::styled("● Healthy", Style::default().fg(t.success))
            } else {
                Span::styled("● Unhealthy", Style::default().fg(t.error))
            },
        ]),
        Line::from(vec![
            Span::styled("Enabled: ", Style::default().fg(t.fg_muted)),
            Span::styled(
                if provider.enabled { "Yes" } else { "No" },
                Style::default().fg(if provider.enabled { t.success } else { t.fg_muted }),
            ),
        ]),
        Line::from(vec![
            Span::styled("URL: ", Style::default().fg(t.fg_muted)),
            Span::styled(&provider.base_url, Style::default().fg(t.accent)),
        ]),
        Line::from(vec![
            Span::styled("Priority: ", Style::default().fg(t.fg_muted)),
            Span::styled(provider.priority.to_string(), Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("Weight: ", Style::default().fg(t.fg_muted)),
            Span::styled(format!("{:.2}", provider.weight), Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Requests: ", Style::default().fg(t.fg_muted)),
            Span::styled(provider.request_count.to_string(), Style::default().fg(t.info)),
            Span::styled(" │ Errors: ", Style::default().fg(t.fg_muted)),
            Span::styled(provider.error_count.to_string(), Style::default().fg(t.error)),
        ]),
        Line::from(vec![
            Span::styled("Avg Latency: ", Style::default().fg(t.fg_muted)),
            Span::styled(format!("{:.2}ms", provider.avg_latency_ms), Style::default().fg(t.fg_secondary)),
        ]),
    ];

    let info = Paragraph::new(info_text).wrap(Wrap { trim: true });
    frame.render_widget(info, chunks[0]);

    // Models list
    let model_items: Vec<ListItem> = provider.models.iter().map(|m| {
        ListItem::new(Line::from(vec![
            Span::styled("  • ", Style::default().fg(t.fg_muted)),
            Span::styled(m, Style::default().fg(t.fg_secondary)),
        ]))
    }).collect();

    let models_list = List::new(model_items)
        .block(Block::default()
            .title(Span::styled(
                format!(" Models ({}) ", provider.models.len()),
                Style::default().fg(t.fg_muted),
            ))
            .borders(Borders::TOP)
            .border_style(Style::default().fg(t.border)));

    frame.render_widget(models_list, chunks[1]);
}

// ============================================================================
// Models View
// ============================================================================

fn draw_models(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let models = state.models.lock().unwrap();
    
    let rows: Vec<Row> = models.iter().enumerate().map(|(idx, m)| {
            let is_selected = idx == state.selected_index;
            let style = if is_selected {
            Style::default().bg(t.selection)
            } else {
                Style::default()
            };

        let status_icon = if m.enabled {
            Span::styled("●", Style::default().fg(t.success))
        } else {
            Span::styled("○", Style::default().fg(t.fg_muted))
        };

            Row::new(vec![
            Cell::from(status_icon),
            Cell::from(Span::styled(&m.id, Style::default().fg(t.fg_primary))),
            Cell::from(Span::styled(&m.provider, Style::default().fg(t.accent))),
            Cell::from(Span::styled(
                m.context_length.map(|c| c.to_string()).unwrap_or_else(|| "-".to_string()),
                Style::default().fg(t.fg_secondary),
            )),
            Cell::from(Span::styled(
                m.request_count.to_string(),
                Style::default().fg(t.info),
            )),
            Cell::from(Span::styled(
                format!("{:.0}ms", m.avg_latency_ms),
                Style::default().fg(t.fg_secondary),
            )),
            Cell::from(Span::styled(
                m.capabilities.join(", "),
                Style::default().fg(t.fg_muted),
            )),
        ]).style(style)
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Min(25),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(20),
        ],
    )
        .header(
        Row::new(vec!["", "Model ID", "Provider", "Context", "Requests", "Latency", "Capabilities"])
            .style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
    )
    .block(Block::default()
        .title(Span::styled(
            format!(" 󰘬 Models ({}) ", models.len()),
            Style::default().fg(t.fg_secondary),
        ))
                .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if state.focus_area == FocusArea::Content { t.border_focus } else { t.border }
        ))
        .style(Style::default().bg(t.bg_secondary)))
    .highlight_style(Style::default().bg(t.selection));

    let mut area_with_margin = area;
    area_with_margin.x += 1;
    area_with_margin.y += 1;
    area_with_margin.width = area_with_margin.width.saturating_sub(2);
    area_with_margin.height = area_with_margin.height.saturating_sub(2);
    
    frame.render_widget(table, area_with_margin);
}

// ============================================================================
// Requests View
// ============================================================================

fn draw_requests(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let requests = state.recent_requests.lock().unwrap();
    
    let rows: Vec<Row> = requests.iter().rev()
        .skip(state.scroll_position)
        .take(area.height.saturating_sub(5) as usize)
        .enumerate()
        .map(|(idx, r)| {
            let is_selected = idx == state.selected_index.saturating_sub(state.scroll_position);
            let style = if is_selected {
                Style::default().bg(t.selection)
            } else {
                Style::default()
            };

            let status_color = if r.status_code < 300 {
                t.success
            } else if r.status_code < 500 {
                t.warning
            } else {
                t.error
            };

            Row::new(vec![
                Cell::from(Span::styled(
                    r.timestamp.format("%H:%M:%S%.3f").to_string(),
                    Style::default().fg(t.fg_muted),
                )),
                Cell::from(Span::styled(&r.method, Style::default().fg(t.accent))),
                Cell::from(Span::styled(&r.endpoint, Style::default().fg(t.fg_secondary))),
                Cell::from(Span::styled(
                    r.model.clone().unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(t.fg_primary),
                )),
                Cell::from(Span::styled(
                    r.status_code.to_string(),
                    Style::default().fg(status_color),
                )),
                Cell::from(Span::styled(
                    format!("{:.0}ms", r.latency_ms),
                    Style::default().fg(t.fg_secondary),
                )),
                Cell::from(Span::styled(
                    r.tokens.map(|t| t.to_string()).unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(t.info),
                )),
                Cell::from(Span::styled(
                    r.error.clone().unwrap_or_default(),
                    Style::default().fg(t.error),
                )),
            ]).style(style)
        }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(15),
            Constraint::Length(6),
            Constraint::Min(20),
            Constraint::Length(25),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(15),
        ],
    )
        .header(
        Row::new(vec!["Timestamp", "Method", "Endpoint", "Model", "Code", "Latency", "Tokens", "Error"])
            .style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
    )
    .block(Block::default()
        .title(Span::styled(
            format!(" 󰑐 Request Monitor ({}/{}) ", 
                state.scroll_position + 1,
                    requests.len()
            ),
            Style::default().fg(t.fg_secondary),
                ))
                .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(
            if state.focus_area == FocusArea::Content { t.border_focus } else { t.border }
        ))
        .style(Style::default().bg(t.bg_secondary)));

    let mut area_with_margin = area;
    area_with_margin.x += 1;
    area_with_margin.y += 1;
    area_with_margin.width = area_with_margin.width.saturating_sub(2);
    area_with_margin.height = area_with_margin.height.saturating_sub(2);
    
    frame.render_widget(table, area_with_margin);
}

// ============================================================================
// Agents View (MoA)
// ============================================================================

fn draw_agents(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .margin(1)
        .split(area);

    let agents = state.agents.lock().unwrap();
    
    let items: Vec<ListItem> = agents.iter().enumerate().map(|(idx, a)| {
            let is_selected = idx == state.selected_index;
        
            let status_color = match a.status.as_str() {
            "active" => t.success,
            "idle" => t.warning,
            "error" => t.error,
            _ => t.fg_muted,
            };

            let style = if is_selected {
            Style::default().bg(t.selection)
            } else {
                Style::default()
            };

            ListItem::new(vec![
                Line::from(vec![
                Span::styled("● ", Style::default().fg(status_color)),
                Span::styled(&a.name, Style::default().fg(t.fg_primary).add_modifier(Modifier::BOLD)),
                Span::styled(" [", Style::default().fg(t.fg_muted)),
                Span::styled(&a.agent_type, Style::default().fg(t.accent)),
                Span::styled("]", Style::default().fg(t.fg_muted)),
                ]),
                Line::from(vec![
                Span::styled("  Strategy: ", Style::default().fg(t.fg_muted)),
                Span::styled(&a.strategy, Style::default().fg(t.info)),
            ]),
            Line::from(vec![
                Span::styled("  Requests: ", Style::default().fg(t.fg_muted)),
                Span::styled(a.request_count.to_string(), Style::default().fg(t.fg_secondary)),
                Span::styled(" │ Success: ", Style::default().fg(t.fg_muted)),
                    Span::styled(
                    format!("{:.1}%", a.success_rate * 100.0),
                    Style::default().fg(if a.success_rate > 0.95 { t.success } else { t.warning }),
                ),
            ]),
        ]).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(
                format!(" 󰯙 MoA Agents ({}) ", agents.len()),
                Style::default().fg(t.fg_secondary),
            ))
                .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if state.focus_area == FocusArea::Content { t.border_focus } else { t.border }
            ))
            .style(Style::default().bg(t.bg_secondary)));

    frame.render_widget(list, chunks[0]);

    // Agent details
    if let Some(agent) = agents.get(state.selected_index) {
        let detail_text = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(t.fg_muted)),
                Span::styled(&agent.id, Style::default().fg(t.fg_secondary)),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().fg(t.fg_muted)),
                Span::styled(&agent.agent_type, Style::default().fg(t.accent)),
            ]),
            Line::from(vec![
                Span::styled("Strategy: ", Style::default().fg(t.fg_muted)),
                Span::styled(&agent.strategy, Style::default().fg(t.info)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(t.fg_muted)),
                Span::styled(&agent.status, Style::default().fg(
                    match agent.status.as_str() {
                        "active" => t.success,
                        "idle" => t.warning,
                        _ => t.error,
                    }
                )),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("═══ Performance ═══", Style::default().fg(t.fg_muted)),
            ]),
            Line::from(vec![
                Span::styled("Requests: ", Style::default().fg(t.fg_muted)),
                Span::styled(agent.request_count.to_string(), Style::default().fg(t.fg_secondary)),
            ]),
            Line::from(vec![
                Span::styled("Success Rate: ", Style::default().fg(t.fg_muted)),
                Span::styled(
                    format!("{:.2}%", agent.success_rate * 100.0),
                    Style::default().fg(if agent.success_rate > 0.95 { t.success } else { t.warning }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Avg Latency: ", Style::default().fg(t.fg_muted)),
                Span::styled(format!("{:.2}ms", agent.avg_latency_ms), Style::default().fg(t.fg_secondary)),
            ]),
        ];

        let details = Paragraph::new(detail_text)
            .block(Block::default()
                .title(Span::styled(" 󰋽 Agent Details ", Style::default().fg(t.fg_secondary)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(t.border))
                .style(Style::default().bg(t.bg_secondary)))
            .wrap(Wrap { trim: true });

        frame.render_widget(details, chunks[1]);
    }
}

// ============================================================================
// Logs View
// ============================================================================

fn draw_logs(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let logs = state.logs.lock().unwrap();
    let log_filter = state.log_filter.lock().unwrap();
    
    let items: Vec<ListItem> = logs.iter().rev()
        .filter(|log| {
            log_filter.is_empty() || 
            log.message.to_lowercase().contains(&log_filter.to_lowercase()) ||
            log.target.to_lowercase().contains(&log_filter.to_lowercase())
        })
        .skip(state.scroll_position)
        .take(area.height.saturating_sub(4) as usize)
        .map(|log| {
            let level_color = match log.level.as_str() {
                "ERROR" => t.error,
                "WARN" => t.warning,
                "INFO" => t.info,
                "DEBUG" => t.fg_muted,
                "TRACE" => t.fg_muted,
                _ => t.fg_secondary,
            };

            let level_icon = match log.level.as_str() {
                "ERROR" => "󰅚",
                "WARN" => "󰀦",
                "INFO" => "󰋽",
                "DEBUG" => "󰃤",
                "TRACE" => "󰔷",
                _ => "•",
            };

            ListItem::new(Line::from(vec![
                    Span::styled(
                        log.timestamp.format("%H:%M:%S%.3f").to_string(),
                    Style::default().fg(t.fg_muted),
                ),
                Span::raw(" "),
                Span::styled(level_icon, Style::default().fg(level_color)),
                Span::styled(
                    format!(" {:5} ", log.level),
                    Style::default().fg(level_color),
                ),
                Span::styled(&log.target, Style::default().fg(t.accent).add_modifier(Modifier::DIM)),
                Span::styled(" │ ", Style::default().fg(t.border)),
                Span::styled(&log.message, Style::default().fg(t.fg_secondary)),
            ]))
        }).collect();

    let title = if log_filter.is_empty() {
        format!(" 󰌱 Logs ({}) ", logs.len())
    } else {
        format!(" 󰌱 Logs (filter: '{}') ", log_filter)
    };

    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(title, Style::default().fg(t.fg_secondary)))
                .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if state.focus_area == FocusArea::Content { t.border_focus } else { t.border }
            ))
            .style(Style::default().bg(t.bg_secondary)));

    let mut area_with_margin = area;
    area_with_margin.x += 1;
    area_with_margin.y += 1;
    area_with_margin.width = area_with_margin.width.saturating_sub(2);
    area_with_margin.height = area_with_margin.height.saturating_sub(2);
    
    frame.render_widget(list, area_with_margin);
}

// ============================================================================
// Settings View
// ============================================================================

fn draw_settings(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let config_sections = state.config_sections.lock().unwrap();
    
    let items: Vec<ListItem> = config_sections.iter().enumerate().flat_map(|(section_idx, section)| {
        let mut result = vec![];
        
        // Section header
        let is_section_selected = section_idx == state.selected_index;
        let header_style = if is_section_selected {
            Style::default().bg(t.selection)
        } else {
            Style::default()
        };

        let expand_icon = if section.expanded { "▼" } else { "▶" };
        
        result.push(ListItem::new(Line::from(vec![
            Span::styled(expand_icon, Style::default().fg(t.accent)),
            Span::raw(" "),
            Span::styled(&section.name, Style::default().fg(t.fg_primary).add_modifier(Modifier::BOLD)),
        ])).style(header_style));

        // Section items if expanded
        if section.expanded {
            for item in &section.items {
                result.push(ListItem::new(Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(&item.key, Style::default().fg(t.fg_muted)),
                    Span::styled(": ", Style::default().fg(t.fg_muted)),
                    Span::styled(&item.value, Style::default().fg(
                        if item.editable { t.accent } else { t.fg_secondary }
                    )),
                    if item.editable {
                        Span::styled(" [edit]", Style::default().fg(t.fg_muted).add_modifier(Modifier::DIM))
                    } else {
                        Span::raw("")
                    },
                ])));
            }
        }

        result
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .title(Span::styled(" 󰒓 Settings ", Style::default().fg(t.fg_secondary)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if state.focus_area == FocusArea::Content { t.border_focus } else { t.border }
            ))
            .style(Style::default().bg(t.bg_secondary)));

    let mut area_with_margin = area;
    area_with_margin.x += 1;
    area_with_margin.y += 1;
    area_with_margin.width = area_with_margin.width.saturating_sub(2);
    area_with_margin.height = area_with_margin.height.saturating_sub(2);
    
    frame.render_widget(list, area_with_margin);
}

// ============================================================================
// Status Bar
// ============================================================================

fn draw_status_bar(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),     // Keys
            Constraint::Length(25), // Refresh
        ])
        .split(area);

    // Keyboard shortcuts
    let shortcuts = match state.input_mode {
        InputMode::Normal => vec![
            ("q", "Quit"),
            ("Tab", "Switch"),
            ("↑↓", "Navigate"),
            ("Enter", "Select"),
            ("r", "Refresh"),
            ("/", "Search"),
            ("?", "Help"),
        ],
        InputMode::Search => vec![
            ("Esc", "Cancel"),
            ("Enter", "Apply"),
        ],
        InputMode::Editing => vec![
            ("Esc", "Cancel"),
            ("Enter", "Save"),
        ],
        InputMode::Command => vec![
            ("Esc", "Cancel"),
            ("Enter", "Execute"),
        ],
    };

    let shortcut_spans: Vec<Span> = shortcuts.iter().flat_map(|(key, desc)| {
        vec![
            Span::styled(*key, Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {} ", desc), Style::default().fg(t.fg_muted)),
            Span::styled("│ ", Style::default().fg(t.border)),
        ]
    }).collect();

    let shortcuts_bar = Paragraph::new(Line::from(shortcut_spans))
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(t.border))
            .style(Style::default().bg(t.bg_secondary)));

    frame.render_widget(shortcuts_bar, chunks[0]);

    // Refresh info
    let last_refresh = state.last_refresh.read().unwrap();
    let refresh_text = if let Some(time) = last_refresh.as_ref() {
        format!("󰑓 {}", time.format("%H:%M:%S"))
    } else {
        "󰑓 Never".to_string()
    };

    let auto_text = if state.auto_refresh {
        format!(" ({}s)", state.refresh_interval)
    } else {
        " (off)".to_string()
    };

    let refresh_bar = Paragraph::new(Line::from(vec![
        Span::styled(refresh_text, Style::default().fg(t.fg_muted)),
        Span::styled(auto_text, Style::default().fg(t.fg_muted).add_modifier(Modifier::DIM)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border))
        .style(Style::default().bg(t.bg_secondary)));

    frame.render_widget(refresh_bar, chunks[1]);
}

// ============================================================================
// Help Overlay
// ============================================================================

fn draw_help_overlay(frame: &mut Frame, _state: &AppState, t: &Theme) {
    let area = centered_rect(60, 70, frame.size());
    
    frame.render_widget(Clear, area);
    
    let help_text = vec![
        Line::from(vec![
            Span::styled("⚡ GaussMeridian TUI Help", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("═══ Navigation ═══", Style::default().fg(t.fg_muted))]),
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab  ", Style::default().fg(t.accent)),
            Span::styled("Switch between views", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("  1-7              ", Style::default().fg(t.accent)),
            Span::styled("Jump to specific view", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓ or j/k       ", Style::default().fg(t.accent)),
            Span::styled("Navigate items", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("  PgUp/PgDn        ", Style::default().fg(t.accent)),
            Span::styled("Page scroll", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("  Enter            ", Style::default().fg(t.accent)),
            Span::styled("Select/activate item", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("═══ Actions ═══", Style::default().fg(t.fg_muted))]),
        Line::from(vec![
            Span::styled("  r                ", Style::default().fg(t.accent)),
            Span::styled("Refresh data", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("  /                ", Style::default().fg(t.accent)),
            Span::styled("Search/filter", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("  e                ", Style::default().fg(t.accent)),
            Span::styled("Toggle enable/disable", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("  d                ", Style::default().fg(t.accent)),
            Span::styled("View details", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("═══ General ═══", Style::default().fg(t.fg_muted))]),
        Line::from(vec![
            Span::styled("  ?                ", Style::default().fg(t.accent)),
            Span::styled("Toggle this help", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("  q                ", Style::default().fg(t.accent)),
            Span::styled("Quit application", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(vec![
            Span::styled("  Esc              ", Style::default().fg(t.accent)),
            Span::styled("Cancel/close", Style::default().fg(t.fg_secondary)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(t.fg_muted)),
            Span::styled("Esc", Style::default().fg(t.accent)),
            Span::styled(" or ", Style::default().fg(t.fg_muted)),
            Span::styled("?", Style::default().fg(t.accent)),
            Span::styled(" to close this help", Style::default().fg(t.fg_muted)),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default()
            .title(Span::styled(" Help ", Style::default().fg(t.fg_secondary)))
                .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(t.border_focus))
            .style(Style::default().bg(t.bg_primary)))
        .wrap(Wrap { trim: false });

    frame.render_widget(help, area);
}

// ============================================================================
// Notifications
// ============================================================================

fn draw_notifications(frame: &mut Frame, state: &AppState, t: &Theme) {
    let notifications = state.notifications.lock().unwrap();
    let now = chrono::Utc::now();
    
    // Filter active notifications
    let active: Vec<_> = notifications.iter()
        .filter(|n| {
            let elapsed = (now - n.timestamp).num_seconds() as u64;
            elapsed < n.duration_secs
        })
        .take(3)
        .collect();

    if active.is_empty() {
        return;
    }

    // Position in bottom-right corner
    let area = Rect {
        x: frame.size().width.saturating_sub(45),
        y: frame.size().height.saturating_sub(4 + (active.len() as u16 * 3)),
        width: 42,
        height: (active.len() as u16 * 3) + 1,
    };

    for (idx, notification) in active.iter().enumerate() {
        let y_offset = idx as u16 * 3;
        let notif_area = Rect {
            x: area.x,
            y: area.y + y_offset,
            width: area.width,
            height: 3,
        };

        let (icon, color) = match notification.level {
            NotificationLevel::Info => ("󰋽", t.info),
            NotificationLevel::Success => ("󰄬", t.success),
            NotificationLevel::Warning => ("󰀦", t.warning),
            NotificationLevel::Error => ("󰅚", t.error),
        };

        frame.render_widget(Clear, notif_area);
        
        let notif = Paragraph::new(Line::from(vec![
            Span::styled(icon, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(&notification.message, Style::default().fg(t.fg_primary)),
        ]))
        .block(Block::default()
                .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(color))
            .style(Style::default().bg(t.bg_secondary)));

        frame.render_widget(notif, notif_area);
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Format duration for display
pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Format bytes for display
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
