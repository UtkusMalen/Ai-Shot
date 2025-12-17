use ai_shot_core::{
    capture::ScreenCapturer,
    config::Config,
    gemini::GeminiClient,
    image_processing::ImageProcessor,
    init,
    ui
};
use anyhow::{Context, Result};
use std::io;
use std::io::Write;
use arboard::Clipboard;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Prompt to send to Gemini
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,

    /// Override the model defined in .env
    #[arg(short, long)]
    model: Option<String>,

    /// Copy the result to clipboard automatically
    #[arg(short, long, default_value_t = false)]
    copy: bool,

    /// Save the debug screenshot to disk
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup
    let _ = dotenvy::dotenv();
    init();
    let args = Args::parse();

    // Load config and override model if specified via CLI
    let mut config = Config::load().context("Failed to load configuration")?;
    if let Some(m) = args.model {
        config.model_name = m;
    }

    // Determine prompt
    // Join all trailing arguments into one string
    let mut prompt_text = args.prompt.join(" ");

    // If no prompt provided via CLI, capture screen first, then ask

    // Capture screen
    let capturer = ScreenCapturer::new().context("Failed to initialize screen capturer")?;
    let screenshot = capturer.capture_screen().context("Failed to capture screen")?;

    // Selection UI
    let selection_result = ui::run_selection_ui(screenshot.clone())?;

    match selection_result {
        Some((rect, ui_size)) => {
            // Processing
            let base64_img = ImageProcessor::process_selection(&screenshot, rect, ui_size)
                .context("Failed to process selection")?;

            if args.debug {
                println!("Debug: Image processed. Size: {} chars", base64_img.len());
            }

            // If prompt was empty, ask now
            if prompt_text.trim().is_empty() {
                print!("Enter prompt (default: 'Explain this'): ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                prompt_text = input.trim().to_string();

                if prompt_text.is_empty() {
                    prompt_text = "Explain what is in this image in detail.".to_string();
                }
            }

            // Send to API
            println!("Analyzing with {}...", config.model_name);
            let client = GeminiClient::new(&config);

            match client?.analyze_image(base64_img, prompt_text).await {
                Ok(response) => {
                    // Print to Stdout
                    println!("\n---\n{}\n---", response);

                    // Copy to clipboard if requested
                    if args.copy {
                        match Clipboard::new() {
                            Ok(mut clipboard) => {
                                if let Err(e) = clipboard.set_text(response.clone()) {
                                    eprintln!("Warning: Failed to copy to clipboard: {}", e);
                                } else {
                                    println!("(Copied to clipboard)");
                                }
                            },
                            Err(e) => eprintln!("Warning: Could not access clipboard: {}", e),
                        }
                    }
                }
                Err(e) => eprintln!("Gemini API Error: {}", e),
            }
        }
        None => {
            println!("Selection cancelled");
        }
    }

    Ok(())
}