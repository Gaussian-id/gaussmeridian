//! Cost calculation utilities for providers

use gaussmeridian_models::CostInfo;

/// Cost calculation utilities
pub struct CostCalculator;

impl CostCalculator {
    /// Calculate cost for a request based on token usage
    pub fn calculate_cost(input_tokens: u32, output_tokens: u32, cost_info: &CostInfo) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * cost_info.input_cost_per_1k_tokens;
        let output_cost = (output_tokens as f64 / 1000.0) * cost_info.output_cost_per_1k_tokens;
        input_cost + output_cost
    }

    /// Get cost info for a specific model
    pub fn get_model_cost_info(model: &str) -> Option<CostInfo> {
        match model {
            "gpt-4" => Some(CostInfo {
                input_cost_per_1k_tokens: 0.03,
                output_cost_per_1k_tokens: 0.06,
                currency: "USD".to_string(),
                model: model.to_string(),
            }),
            "gpt-4-turbo" => Some(CostInfo {
                input_cost_per_1k_tokens: 0.01,
                output_cost_per_1k_tokens: 0.03,
                currency: "USD".to_string(),
                model: model.to_string(),
            }),
            "gpt-3.5-turbo" => Some(CostInfo {
                input_cost_per_1k_tokens: 0.0015,
                output_cost_per_1k_tokens: 0.002,
                currency: "USD".to_string(),
                model: model.to_string(),
            }),
            "claude-3-opus" => Some(CostInfo {
                input_cost_per_1k_tokens: 0.015,
                output_cost_per_1k_tokens: 0.075,
                currency: "USD".to_string(),
                model: model.to_string(),
            }),
            "claude-3-sonnet" => Some(CostInfo {
                input_cost_per_1k_tokens: 0.003,
                output_cost_per_1k_tokens: 0.015,
                currency: "USD".to_string(),
                model: model.to_string(),
            }),
            "claude-3-haiku" => Some(CostInfo {
                input_cost_per_1k_tokens: 0.00025,
                output_cost_per_1k_tokens: 0.00125,
                currency: "USD".to_string(),
                model: model.to_string(),
            }),
            _ => None,
        }
    }

    /// Estimate token count for text (rough approximation)
    pub fn estimate_tokens(text: &str) -> u32 {
        // Rough approximation: 1 token ≈ 4 characters for English text
        (text.len() as f64 / 4.0).ceil() as u32
    }

    /// Estimate tokens for messages
    pub fn estimate_message_tokens(messages: &[gaussmeridian_models::Message]) -> u32 {
        messages
            .iter()
            .map(|msg| {
                match &msg.content {
                    gaussmeridian_models::Content::Text(text) => Self::estimate_tokens(text),
                    gaussmeridian_models::Content::Parts(parts) => {
                        parts
                            .iter()
                            .map(|part| {
                                match part {
                                    gaussmeridian_models::ContentPart::Text { text } => {
                                        Self::estimate_tokens(text)
                                    }
                                    gaussmeridian_models::ContentPart::ImageUrl { .. } => 85, // Rough estimate for image
                                }
                            })
                            .sum()
                    }
                }
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let cost_info = CostInfo {
            input_cost_per_1k_tokens: 0.01,
            output_cost_per_1k_tokens: 0.03,
            currency: "USD".to_string(),
            model: "test-model".to_string(),
        };

        let cost = CostCalculator::calculate_cost(1000, 500, &cost_info);
        assert_eq!(cost, 0.01 + 0.015); // 0.025
    }

    #[test]
    fn test_token_estimation() {
        let tokens = CostCalculator::estimate_tokens("Hello world");
        assert!(tokens > 0);
    }
}
