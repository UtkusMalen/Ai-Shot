use crate::config::Config;
use crate::error::{AppError, Result};
use gemini_rust::{Gemini, Content, Part, Role, Blob, Message};

pub struct GeminiClient {
    client: Gemini,
}

impl GeminiClient {
    pub fn new(config: &Config) -> Result<Self> {
        // Initialize the client with the API key and model, explicitly setting the base URL to avoid BadScheme error
        let base_url = url::Url::parse("https://generativelanguage.googleapis.com/v1beta/")
            .map_err(|e| AppError::Config(format!("Invalid base URL: {}", e)))?;

        let model_url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}", config.model_name);

        let client = Gemini::with_model_and_base_url(&config.gemini_api_key, model_url, base_url)
            .map_err(|e| AppError::Config(format!("Failed to create Gemini client: {}", e)))?;

        Ok(Self {
            client,
        })
    }

    /// Sends an image and a text prompt to the Gemini API
    pub async fn analyze_image(&self, base64_image: String, prompt: String) -> Result<String> {
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

        // Send request
        let response = self.client
            .generate_content()
            .with_messages(vec![message])
            .execute()
            .await
            .map_err(|e| AppError::GeminiApi(format!("API request failed: {:?}", e)))?;

        // Parse Response
        if let Some(candidate) = response.candidates.first() {
            let content = &candidate.content;
            if let Some(parts) = &content.parts {
                    // Match against the struct variant Part::Text { text, .. }
                    if let Some(Part::Text { text, .. }) = parts.first() {
                        return Ok(text.clone());
                    }
            }
        }

        Err(AppError::GeminiApi("No text response received from Gemini".to_string()))
    }
}