//! Event handling for keyboard and mouse input
//!
//! Provides async event handling with proper error recovery
//! and configurable tick rate for smooth UI updates.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Event types that can be handled by the application
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Keyboard event
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Tick event for periodic updates
    Tick,
    /// Refresh event to update data
    Refresh,
    /// Quit event
    Quit,
    /// Error event
    Error(String),
}

/// Event handler that manages input events
pub struct EventHandler {
    /// Receiver for events
    rx: mpsc::Receiver<AppEvent>,
    /// Sender for events (kept for shutdown)
    _tx: mpsc::Sender<AppEvent>,
}

impl EventHandler {
    /// Create a new event handler with specified tick rate
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel(256);

        // Spawn event loop task
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let mut last_tick = Instant::now();

            loop {
                // Calculate timeout until next tick
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::ZERO);

                // Poll for events
                match event::poll(timeout) {
                    Ok(true) => {
                        // Event available
                        match event::read() {
                            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                                if tx_clone.send(AppEvent::Key(key)).await.is_err() {
                                    break;
                                }
                            }
                            Ok(Event::Mouse(mouse)) => {
                                if tx_clone.send(AppEvent::Mouse(mouse)).await.is_err() {
                                    break;
                                }
                            }
                            Ok(Event::Resize(_, _)) => {
                                // Terminal resize - trigger refresh
                                if tx_clone.send(AppEvent::Tick).await.is_err() {
                                    break;
                                }
                            }
                            Ok(_) => {
                                // Other events ignored
                            }
                            Err(e) => {
                                let _ = tx_clone
                                    .send(AppEvent::Error(format!("Event read error: {}", e)))
                                    .await;
                            }
                        }
                    }
                    Ok(false) => {
                        // No event, continue to tick check
                    }
                    Err(e) => {
                        let _ = tx_clone
                            .send(AppEvent::Error(format!("Event poll error: {}", e)))
                            .await;
                    }
                }

                // Send tick event if interval elapsed
                if last_tick.elapsed() >= tick_rate {
                    if tx_clone.send(AppEvent::Tick).await.is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self { rx, _tx: tx }
    }

    /// Get the next event (async, blocks until event available)
    pub async fn next(&mut self) -> AppEvent {
        self.rx.recv().await.unwrap_or(AppEvent::Quit)
    }
}

/// Helper function to check if a key event matches a specific key combination
#[inline]
pub fn matches_key(key: &KeyEvent, code: KeyCode, modifiers: KeyModifiers) -> bool {
    key.code == code && key.modifiers == modifiers
}

/// Helper function to check if a key event is a navigation key
#[inline]
pub fn is_navigation_key(key: &KeyEvent) -> bool {
    matches!(
        key.code,
        KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
    )
}

/// Helper function to check if a key event is a quit key
#[inline]
pub fn is_quit_key(key: &KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('q'), KeyModifiers::NONE)
            | (KeyCode::Char('c'), KeyModifiers::CONTROL)
    )
}

/// Helper function to check if a key event is a confirm key
#[inline]
pub fn is_confirm_key(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Enter)
}

/// Helper function to check if a key event is a cancel key
#[inline]
pub fn is_cancel_key(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_key() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(matches_key(&key, KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!matches_key(&key, KeyCode::Char('q'), KeyModifiers::CONTROL));
    }

    #[test]
    fn test_is_navigation_key() {
        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        
        assert!(is_navigation_key(&up));
        assert!(!is_navigation_key(&q));
    }

    #[test]
    fn test_is_quit_key() {
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        
        assert!(is_quit_key(&q));
        assert!(is_quit_key(&ctrl_c));
        assert!(!is_quit_key(&enter));
    }
}
