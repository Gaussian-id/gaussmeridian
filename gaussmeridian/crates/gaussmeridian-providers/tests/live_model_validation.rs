//! Live per-model adapter validation (opt-in, network + real API keys).
//!
//! Run explicitly with:
//!   cargo test -p gaussmeridian-providers --test live_model_validation -- --ignored --nocapture
//!
//! Validates that each catalog model on a *registered* provider (openai, anthropic)
//! actually completes through the REAL adapter code path (`OpenAIProvider` /
//! `AnthropicProvider::chat_completion`) against the live provider APIs, using
//! `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` from the environment. These are exactly
//! the models the router can't exercise individually end-to-end, because selection
//! ignores the requested model and cheapest-first ordering means only gpt-4o-mini
//! is ever reached while OpenAI is healthy.

use gaussmeridian_models::ChatCompletionRequest;
use gaussmeridian_providers::{
    AnthropicConfig, AnthropicProvider, LLMProvider, OpenAIConfig, OpenAIProvider,
};

fn tiny_request(model: &str) -> ChatCompletionRequest {
    serde_json::from_value(serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": "Reply with exactly: WORKING" }],
        "max_tokens": 16
    }))
    .expect("request json is valid")
}

fn env_key(name: &str) -> String {
    match std::env::var(name) {
        Ok(v) if !v.is_empty() => v,
        _ => panic!("{name} must be set for live validation"),
    }
}

async fn assert_model_works<P>(provider: &P, model: &str)
where
    P: LLMProvider,
    P::Error: std::fmt::Display,
{
    let resp = provider
        .chat_completion(tiny_request(model))
        .await
        .unwrap_or_else(|e| panic!("[{model}] adapter call failed: {e}"));

    let content = serde_json::to_string(&resp.choices).unwrap_or_default();
    println!(
        "[{model}] OK — served_as={} usage={:?} content_excerpt={}",
        resp.model,
        resp.usage,
        &content.chars().take(120).collect::<String>()
    );
    assert!(
        !resp.choices.is_empty(),
        "[{model}] returned no choices"
    );
}

#[tokio::test]
#[ignore = "live network test — requires OPENAI_API_KEY"]
async fn openai_catalog_models_complete() {
    // dotenvy so the test finds the same .env the server uses.
    let _ = dotenvy::from_path("../../.env");
    let provider = OpenAIProvider::new(OpenAIConfig::new(env_key("OPENAI_API_KEY")));
    for model in ["gpt-4o", "gpt-4o-mini", "o4-mini"] {
        assert_model_works(&provider, model).await;
    }
}

#[tokio::test]
#[ignore = "live network test — requires ANTHROPIC_API_KEY"]
async fn anthropic_catalog_models_complete() {
    let _ = dotenvy::from_path("../../.env");
    let mut config = AnthropicConfig::default();
    config.base_config.api_key = env_key("ANTHROPIC_API_KEY");
    let provider = AnthropicProvider::new(config);
    for model in ["claude-sonnet-4-5", "claude-haiku-4-5"] {
        assert_model_works(&provider, model).await;
    }
}
