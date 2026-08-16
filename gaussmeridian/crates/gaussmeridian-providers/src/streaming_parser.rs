//! Common streaming parser utilities for SSE (Server-Sent Events) format
//!
//! This module provides reusable parsing logic for OpenAI-compatible streaming responses
//! to reduce code duplication across providers.

use futures::{Stream, StreamExt};
use gaussmeridian_models::{ChatCompletionChunk, CompletionChunk, ProviderError};
use std::pin::Pin;

/// Parse SSE stream chunks into structured data
pub struct SSEParser;

impl SSEParser {
    /// Parse a single SSE line into JSON value
    pub fn parse_sse_line(line: &str) -> Option<serde_json::Value> {
        if !line.starts_with("data: ") {
            return None;
        }

        let data = &line[6..]; // Skip "data: " prefix

        if data == "[DONE]" {
            return None; // Signal end of stream
        }

        serde_json::from_str::<serde_json::Value>(data).ok()
    }

    /// Parse bytes chunk into multiple SSE lines
    pub fn parse_chunk(bytes: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Extract chat completion chunk from JSON
    pub fn extract_chat_chunk(
        json: &serde_json::Value,
        default_id: &str,
        default_model: &str,
    ) -> Result<ChatCompletionChunk, ProviderError> {
        let chunk = ChatCompletionChunk {
            id: json["id"].as_str().unwrap_or(default_id).to_string(),
            object: "chat.completion.chunk".to_string(),
            created: json["created"]
                .as_i64()
                .unwrap_or_else(|| chrono::Utc::now().timestamp()),
            model: json["model"].as_str().unwrap_or(default_model).to_string(),
            choices: serde_json::from_value(json["choices"].clone())
                .map_err(|e| ProviderError::Internal(format!("Failed to parse choices: {}", e)))?,
            usage: None,
            system_fingerprint: json["system_fingerprint"].as_str().map(|s| s.to_string()),
        };

        Ok(chunk)
    }

    /// Extract completion chunk from JSON
    pub fn extract_completion_chunk(
        json: &serde_json::Value,
        default_id: &str,
        default_model: &str,
    ) -> Result<CompletionChunk, ProviderError> {
        let chunk = CompletionChunk {
            id: json["id"].as_str().unwrap_or(default_id).to_string(),
            object: "text_completion.chunk".to_string(),
            created: json["created"]
                .as_i64()
                .unwrap_or_else(|| chrono::Utc::now().timestamp()),
            model: json["model"].as_str().unwrap_or(default_model).to_string(),
            choices: serde_json::from_value(json["choices"].clone())
                .map_err(|e| ProviderError::Internal(format!("Failed to parse choices: {}", e)))?,
            usage: None,
        };

        Ok(chunk)
    }
}

/// Transform a bytes stream into SSE chunks
pub fn parse_sse_stream<T, F>(
    stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    extract_fn: F,
    _default_id: &'static str,
    _default_model: String,
) -> Pin<Box<dyn Stream<Item = Result<T, ProviderError>> + Send>>
where
    F: Fn(&serde_json::Value) -> Result<T, ProviderError> + Send + Sync + 'static,
    T: Send + 'static,
{
    let parsed_stream = stream
        .map(move |chunk_result| {
            let chunk = chunk_result
                .map_err(|e| ProviderError::Internal(format!("Stream chunk error: {}", e)))?;

            let lines = SSEParser::parse_chunk(&chunk);
            let mut results = Vec::new();

            for line in lines {
                if let Some(json) = SSEParser::parse_sse_line(&line) {
                    match extract_fn(&json) {
                        Ok(chunk) => results.push(Ok(chunk)),
                        Err(e) => results.push(Err(e)),
                    }
                }
            }

            Ok(results)
        })
        .filter_map(|result| async move {
            match result {
                Ok(chunks) if !chunks.is_empty() => Some(chunks),
                Ok(_) => None,
                Err(e) => Some(vec![Err(e)]),
            }
        })
        .flat_map(|chunks| futures::stream::iter(chunks));

    Box::pin(parsed_stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_line() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk"}"#;
        let json = SSEParser::parse_sse_line(line);
        assert!(json.is_some());
        assert_eq!(json.unwrap()["id"], "chatcmpl-123");
    }

    #[test]
    fn test_parse_sse_line_done() {
        let line = "data: [DONE]";
        let json = SSEParser::parse_sse_line(line);
        assert!(json.is_none());
    }

    #[test]
    fn test_parse_chunk() {
        let bytes = b"data: {\"id\":\"test\"}\n\ndata: [DONE]\n";
        let lines = SSEParser::parse_chunk(bytes);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("data: "));
    }
}
