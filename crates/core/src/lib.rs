pub mod capture;
pub mod config;
pub mod error;
pub mod ui;
pub mod image_processing;
pub mod gemini;

// Re-export common types
pub use config::Config;
pub use error::{AppError, Result};

pub fn init() {
    println!("Core library initialized");
}