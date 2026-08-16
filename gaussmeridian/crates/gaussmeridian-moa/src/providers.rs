//! Provider seam for MoA agents (Seam 2 of the GaussMoA integration).
//!
//! MoA agents used to embed their own per-provider HTTP clients with hardcoded URLs and a
//! `"DUMMY_API_KEY"` — so they never actually worked against real APIs. Instead, agents now call
//! a single small [`ChatProvider`] interface. **gaussmoa owns the interface**; the caller supplies
//! the implementation (dependency inversion), so gaussmoa needs no dependency on the gateway's
//! provider crate:
//! - The GaussMeridian gateway injects an adapter backed by its shared `gaussmeridian-providers`
//!   registry (real keys, BYOK, the o-series fix, real usage) — the one provider stack.
//! - Standalone/debug mode (`main.rs`) uses the built-in OpenAI-compatible [`HttpChatProvider`].
//! - Tests inject a deterministic mock.

use crate::error::{MoaError, MoaResult};
use async_trait::async_trait;

/// A single-completion provider the MoA agents call. Given a model + prompt, return the text.
#[async_trait]
pub trait ChatProvider: Send + Sync + std::fmt::Debug {
    async fn complete(
        &self,
        model: &str,
        prompt: &str,
        temperature: f32,
        max_tokens: usize,
    ) -> MoaResult<String>;
}

/// Built-in OpenAI-compatible provider for standalone/debug mode. The gateway does **not** use
/// this — it injects its own `gaussmeridian-providers`-backed adapter.
#[derive(Debug, Clone)]
pub struct HttpChatProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl HttpChatProvider {
    /// Build an OpenAI-compatible provider. `base_url` should include the version path,
    /// e.g. `https://api.openai.com/v1`.
    pub fn openai_compatible(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            base_url: base_url.into(),
            api_key: api_key.into(),
        }
    }

    /// Build from the environment (`OPENAI_API_KEY`, optional `OPENAI_API_BASE`).
    pub fn from_env() -> Self {
        let base = std::env::var("OPENAI_API_BASE")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        Self::openai_compatible(base, key)
    }
}

#[async_trait]
impl ChatProvider for HttpChatProvider {
    async fn complete(
        &self,
        model: &str,
        prompt: &str,
        temperature: f32,
        max_tokens: usize,
    ) -> MoaResult<String> {
        let body = serde_json::json!({
            "model": model,
            "messages": [{ "role": "user", "content": prompt }],
            "temperature": temperature,
            "max_tokens": max_tokens,
        });
        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| MoaError::network(format!("chat completion request failed: {e}"), Some(e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let detail = resp.text().await.unwrap_or_default();
            return Err(MoaError::network(
                format!("provider returned {status}: {detail}"),
                None,
            ));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MoaError::network(format!("failed to parse provider response: {e}"), Some(e)))?;

        Ok(data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }
}
