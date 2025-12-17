//! Gemini AI client for image analysis.
//!
//! This module provides a wrapper around the `gemini-rust` crate with
//! support for streaming responses and proper error handling.
//!
//! # Features
//!
//! - Image analysis with text prompts
//! - Streaming responses for real-time display
//! - System prompt support
//! - "Thinking" mode for Gemini 2.0+ models
//! - Google Search grounding
//!
//! # Example
//!
//! ```ignore
//! use ai_shot_core::{Config, GeminiClient};
//!
//! let config = Config::load()?;
//! let client = GeminiClient::new(&config)?;
//!
//! // Simple analysis
//! let response = client.analyze_image(base64_image, "What is this?").await?;
//!
//! // Streaming analysis
//! let mut stream = client.analyze_image_stream(
//!     base64_image,
//!     "Explain this code".to_string(),
//!     String::new(),  // system prompt
//!     false,          // thinking
//!     false,          // google search
//! ).await?;
//!
//! while let Some(events) = stream.next().await {
//!     for event in events? {
//!         println!("{:?}", event);
//!     }
//! }
//! ```

use crate::config::Config;
use crate::error::{AppError, Result};
use gemini_rust::{Blob, Content, Gemini, Message, Part, Role};

/// Client for interacting with Google's Gemini AI API.
///
/// The client is designed to be reused across multiple requests.
/// Creating a client involves network validation, so prefer
/// long-lived instances where possible.
///
/// # Client Reuse
///
/// For best performance, create one client and reuse it:
///
/// ```ignore
/// let client = GeminiClient::new(&config)?;
///
/// // Reuse for multiple requests
/// let response1 = client.analyze_image(img1, "prompt1").await?;
/// let response2 = client.analyze_image(img2, "prompt2").await?;
/// ```
pub struct GeminiClient {
    client: Gemini,
}

/// Events emitted during streaming responses.
///
/// The stream alternates between regular text and "thinking" content
/// when thinking mode is enabled.
#[derive(Debug, Clone)]
pub enum GeminiStreamEvent {
    /// Regular response text content.
    Text(String),
    /// Thinking/reasoning content (when thinking mode is enabled).
    Thought(String),
}

impl GeminiClient {
    /// Creates a new Gemini client with the provided configuration.
    ///
    /// Initializes the HTTP client and validates the model URL.
    ///
    /// # Arguments
    /// * `config` - Configuration containing API key and model name
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Config`] if:
    /// - The base URL is invalid
    /// - Client initialization fails
    pub fn new(config: &Config) -> Result<Self> {
        // Initialize the client with the API key and model
        let base_url = url::Url::parse("https://generativelanguage.googleapis.com/v1beta/")
            .map_err(|e| AppError::config(format!("Invalid base URL: {}", e)))?;

        // Ensure model name has proper prefix
        let model_name = if config.model_name.starts_with("models/") {
            config.model_name.clone()
        } else {
            format!("models/{}", config.model_name)
        };
        let model_url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{}",
            model_name
        );

        let client = Gemini::with_model_and_base_url(&config.gemini_api_key, model_url, base_url)
            .map_err(|e| AppError::config(format!("Failed to create Gemini client: {}", e)))?;

        Ok(Self { client })
    }
    
    /// Sends an image and a text prompt to the Gemini API.
    ///
    /// This is a simple one-shot request that waits for the complete response.
    /// For streaming responses, use [`Self::analyze_image_stream`].
    ///
    /// # Arguments
    /// * `base64_image` - Base64-encoded JPEG image data
    /// * `prompt` - Text prompt describing what to analyze
    ///
    /// # Errors
    ///
    /// Returns [`AppError::GeminiApi`] if:
    /// - The API request fails
    /// - No text response is received
    pub async fn analyze_image(&self, base64_image: String, prompt: String) -> Result<String> {
        let message = self.build_image_message(base64_image, prompt);

        let response = self
            .client
            .generate_content()
            .with_messages(vec![message])
            .execute()
            .await
            .map_err(|e| AppError::gemini(format!("API request failed: {:?}", e)))?;

        // Extract text from response
        if let Some(candidate) = response.candidates.first() {
            if let Some(parts) = &candidate.content.parts {
                for part in parts {
                    if let Part::Text { text, .. } = part {
                        return Ok(text.clone());
                    }
                }
            }
        }

        Err(AppError::gemini("No text response received from Gemini"))
    }
    /// Sends an image and a text prompt to the Gemini API with streaming response.
    ///
    /// Returns a stream of events that can be consumed as they arrive,
    /// enabling real-time display of the response.
    ///
    /// # Arguments
    /// * `base64_image` - Base64-encoded JPEG image data
    /// * `prompt` - Text prompt describing what to analyze
    /// * `system_prompt` - Optional system instructions (empty string to skip)
    /// * `thinking_enabled` - Enable "thinking" mode (Gemini 2.0+ only)
    /// * `google_search` - Enable Google Search grounding
    ///
    /// # Returns
    ///
    /// A pinned stream of [`GeminiStreamEvent`] vectors. Each vector contains
    /// one or more events from a single API response chunk.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::GeminiApi`] if the stream cannot be established.
    pub async fn analyze_image_stream(
        &self,
        base64_image: String,
        prompt: String,
        system_prompt: String,
        thinking_enabled: bool,
        google_search: bool,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<Vec<GeminiStreamEvent>>> + Send>>>
    {
        use futures::TryStreamExt;
        
        // Construct image data blob
        let blob = Blob {
            mime_type: "image/jpeg".to_string(),
            data: base64_image
        };

        // Construct parts
        let image_part = Part::InlineData {
            inline_data: blob,
        };

        let text_part = Part::Text {
            text: prompt,
            thought: None,
            thought_signature: None,
        };

        // Create the content payload
        let content = Content {
            role: Some(Role::User),
            parts: Some(vec![text_part, image_part]),
        };

        // Create the message payload
        let message = Message {
            role: Role::User,
            content,
        };

        // Prepare request builder
        let mut request = self.client.generate_content().with_messages(vec![message]);

        if !system_prompt.trim().is_empty() {
            request = request.with_system_prompt(&system_prompt);
        }

        if thinking_enabled {
            request = request.with_thinking_budget(1024).with_thoughts_included(true);
        }

        if google_search {
            request = request.with_tool(gemini_rust::Tool::google_search());
        }

        // Execute stream
        let stream = request
            .execute_stream()
            .await
            .map_err(|e| AppError::gemini(format!("API request failed: {:?}", e)))?;

        // Convert the Gemini stream into a Stream of Vec<GeminiStreamEvent>
        let mapped_stream = stream
            .map_err(|e| AppError::gemini(format!("Stream error: {:?}", e)))
            .try_filter_map(|response| async move {
                let mut events = Vec::new();

                if let Some(candidate) = response.candidates.first() {
                    if let Some(parts) = &candidate.content.parts {
                        for part in parts {
                            if let Part::Text { text, thought, .. } = part {
                                // Determine if this is thinking content
                                let is_thought = thought.unwrap_or(false);

                                if is_thought {
                                    events.push(GeminiStreamEvent::Thought(text.clone()));
                                } else {
                                    events.push(GeminiStreamEvent::Text(text.clone()));
                                }
                            }
                        }
                    }
                }

                if events.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(events))
                }
            });

        Ok(Box::pin(mapped_stream))
    }

    // ── Private Helper Methods ───────────────────────────────────────────────

    /// Builds a message containing an image and text prompt.
    fn build_image_message(&self, base64_image: String, prompt: String) -> Message {
        let blob = Blob {
            mime_type: "image/jpeg".to_string(),
            data: base64_image,
        };

        let image_part = Part::InlineData { inline_data: blob };
        let text_part = Part::Text {
            text: prompt,
            thought: None,
            thought_signature: None,
        };

        let content = Content {
            role: Some(Role::User),
            parts: Some(vec![text_part, image_part]),
        };

        Message {
            role: Role::User,
            content,
        }
    }
}