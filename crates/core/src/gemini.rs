use crate::config::Config;
use crate::error::{AppError, Result};
use gemini_rust::{Gemini, Content, Part, Role, Blob, Message};

pub struct GeminiClient {
    client: Gemini,
}

#[derive(Debug, Clone)]
pub enum GeminiStreamEvent {
    Text(String),
    Thought(String),
}

impl GeminiClient {
    pub fn new(config: &Config) -> Result<Self> {
        // Initialize the client with the API key and model, explicitly setting the base URL to avoid BadScheme error
        let base_url = url::Url::parse("https://generativelanguage.googleapis.com/v1beta/")
            .map_err(|e| AppError::Config(format!("Invalid base URL: {}", e)))?;

        let model_name = if config.model_name.starts_with("models/") {
            config.model_name.clone()
        } else {
            format!("models/{}", config.model_name)
        };
        let model_url = format!("https://generativelanguage.googleapis.com/v1beta/{}", model_name);

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
    /// Sends an image and a text prompt to the Gemini API and streams the response
    pub async fn analyze_image_stream(&self, base64_image: String, prompt: String, system_prompt: String, thinking_enabled: bool) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<Vec<GeminiStreamEvent>>> + Send>>> {
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

        // Execute stream
        let stream = request
            .execute_stream()
            .await
            .map_err(|e| AppError::GeminiApi(format!("API request failed: {:?}", e)))?;

        // Convert the Gemini stream into a Stream of Vec<GeminiStreamEvent>
        let mapped_stream = stream.map_err(|e| AppError::GeminiApi(format!("Stream error: {:?}", e)))
            .try_filter_map(|response| async move {
                 let mut events = Vec::new();
                 
                 if let Some(candidate) = response.candidates.first() {
                     if let Some(parts) = &candidate.content.parts {
                         for part in parts {
                             match part {
                                 Part::Text { text, thought, .. } => {
                                     // If 'thought' is true/present, treat as thought
                                     // coping with potential Option<bool>
                                     let is_thought = match thought {
                                         Some(t) => *t, // Assumes bool
                                         None => false,
                                     };
                                     
                                     if is_thought {
                                         events.push(GeminiStreamEvent::Thought(text.clone()));
                                     } else {
                                         events.push(GeminiStreamEvent::Text(text.clone()));
                                     }
                                 },
                                 _ => {}
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
}