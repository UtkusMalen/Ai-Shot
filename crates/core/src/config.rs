use std::env;
use crate::error::{AppError, Result};
use dotenvy::dotenv;

#[derive(Clone, Debug)]
pub struct Config {
    pub gemini_api_key: String,
    pub model_name: String
}

impl Config {
    pub fn load() -> Result<Self> {
        // Load .env file if it exists, ignore if it doesn't
        let _ = dotenv();

        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| AppError::Config("GEMINI_API_KEY must be set in environment or .env file".to_string()))?;
        
        let model_name = env::var("GEMINI_MODEL")
            .unwrap_or_else(|_| "gemini-flash-latest".to_string());
        
        Ok(Self {
            gemini_api_key: api_key,
            model_name,
        })
    }
}