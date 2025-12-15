//! AI Validation module for TrinityChain
//! Enhanced for Chat Completion API compatibility, robustness, and safety.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::error::ChainError;

// Claude Haiku 4.5 API endpoint for Chat Completions
const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL_NAME: &str = "claude-3-5-haiku-20241022";
const REQUEST_TIMEOUT: u64 = 30;
const CLAUDE_API_VERSION: &str = "2024-06-01";

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ClaudeApiRequestBody {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Deserialize)]
struct ClaudeApiResponseBody {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

pub struct AIValidator {
    client: Client,
    api_key: String,
}

impl AIValidator {
    pub fn new(api_key: String) -> Self {
        // Initialize client with a timeout to prevent hanging requests
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT))
            .build()
            .unwrap_or_else(|_| Client::new());

        AIValidator {
            client,
            api_key,
        }
    }

    pub async fn validate_transaction(&self, transaction_data: &str) -> Result<bool, ChainError> {
        // Use System/User role separation to prevent prompt injection
        let messages = vec![
            ClaudeMessage {
                role: "user".to_string(),
                content: format!(
                    "You are a validator for the TrinityChain network. Analyze the following transaction data and respond with strictly 'true' if valid or 'false' if invalid. Do not add punctuation or explanation.\n\nTransaction: {}",
                    transaction_data
                ),
            },
        ];

        let request_body = ClaudeApiRequestBody {
            model: MODEL_NAME.to_string(),
            messages,
            max_tokens: 10, // Allow buffer for whitespace/punctuation
        };

        let response = self.client.post(CLAUDE_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", CLAUDE_API_VERSION)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChainError::ApiError(format!("Network request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChainError::ApiError(format!("API Error {}: {}", status, error_text)));
        }

        let response_body: ClaudeApiResponseBody = response.json().await
            .map_err(|e| ChainError::ApiError(format!("JSON parse error: {}", e)))?;

        // Robust parsing logic
        if let Some(content) = response_body.content.first() {
            let text = content.text.trim().to_lowercase();
            // Check for explicit "true" presence, ignoring potential punctuation like "true."
            if text.contains("true") {
                return Ok(true);
            }
        }

        // Default to false for any unclear or negative response
        Ok(false)
    }
}
