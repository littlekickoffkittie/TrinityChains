//! AI Validation module for TrinityChain
//! Enhanced for Chat Completion API compatibility, robustness, and safety.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::error::ChainError;

// Corrected endpoint for Chat Completions
const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/v1/chat/completions";
const MODEL_NAME: &str = "deepseek-coder";
const REQUEST_TIMEOUT: u64 = 10;

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ApiRequestBody {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32, // Controls randomness: 0.0 is deterministic
}

#[derive(Deserialize)]
struct ApiResponseBody {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
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
            ChatMessage {
                role: "system".to_string(),
                content: "You are a validator for the TrinityChain network. Analyze the transaction data. Respond with strictly 'true' if valid or 'false' if invalid. Do not add punctuation or explanation.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: transaction_data.to_string(),
            },
        ];

        let request_body = ApiRequestBody {
            model: MODEL_NAME.to_string(),
            messages,
            max_tokens: 10, // Allow buffer for whitespace/punctuation
            temperature: 0.0, // Ensure deterministic results
        };

        let response = self.client.post(DEEPSEEK_API_URL)
            .bearer_auth(&self.api_key)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChainError::ApiError(format!("Network request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChainError::ApiError(format!("API Error {}: {}", status, error_text)));
        }

        let response_body: ApiResponseBody = response.json().await
            .map_err(|e| ChainError::ApiError(format!("JSON parse error: {}", e)))?;

        // Robust parsing logic
        if let Some(choice) = response_body.choices.first() {
            let content = choice.message.content.trim().to_lowercase();
            // Check for explicit "true" presence, ignoring potential punctuation like "true."
            if content.contains("true") {
                return Ok(true);
            }
        }

        // Default to false for any unclear or negative response
        Ok(false)
    }
}
