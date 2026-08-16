//! Request validation utilities

use gaussmeridian_models::{ChatCompletionRequest, GaussMeridianError};

pub struct RequestValidator;

impl RequestValidator {
    pub fn validate_chat_completion(
        request: &ChatCompletionRequest,
    ) -> Result<(), GaussMeridianError> {
        if request.model.is_empty() {
            return Err(GaussMeridianError::InvalidRequest(
                "Model cannot be empty".to_string(),
            ));
        }

        if request.messages.is_empty() {
            return Err(GaussMeridianError::InvalidRequest(
                "Messages cannot be empty".to_string(),
            ));
        }

        if let Some(max_tokens) = request.max_tokens {
            if max_tokens == 0 {
                return Err(GaussMeridianError::InvalidRequest(
                    "Max tokens cannot be 0".to_string(),
                ));
            }
        }

        if let Some(temperature) = request.temperature {
            if temperature < 0.0 || temperature > 2.0 {
                return Err(GaussMeridianError::InvalidRequest(
                    "Temperature must be between 0.0 and 2.0".to_string(),
                ));
            }
        }

        if let Some(top_p) = request.top_p {
            if top_p < 0.0 || top_p > 1.0 {
                return Err(GaussMeridianError::InvalidRequest(
                    "Top P must be between 0.0 and 1.0".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn sanitize_request(request: &mut ChatCompletionRequest) {
        // Ensure model is lowercase
        request.model = request.model.to_lowercase();

        // Clamp temperature to valid range
        if let Some(temp) = request.temperature {
            request.temperature = Some(temp.clamp(0.0, 2.0));
        }

        // Clamp top_p to valid range
        if let Some(top_p) = request.top_p {
            request.top_p = Some(top_p.clamp(0.0, 1.0));
        }

        // Ensure max_tokens is reasonable
        if let Some(max_tokens) = request.max_tokens {
            request.max_tokens = Some(max_tokens.min(8192));
        }
    }
}
