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
use std::time::Duration;
use arboard::Clipboard;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use termimad::crossterm::style::Color;
use termimad::MadSkin;

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

    /// Select which monitor to capture
    #[arg(long, default_value_t = 0)]
    monitor: usize,

    /// List available monitors and exit
    #[arg(long)]
    list_monitors: bool,
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

    // Initialize capturer
    let capturer = ScreenCapturer::new().context("Failed to initialize screen capturer")?;

    // Handle --list-monitors
    if args.list_monitors {
        println!("Available monitors:");
        for info in capturer.list_screen() {
            println!("{}", info);
        }
        return Ok(());
    }

    // Capture screen
    let screenshot = capturer.capture_screen_by_index(args.monitor)
        .context("Failed to capture screen. Try using --list-monitors to check indices")?;

    // Selection UI
    let selection_result = ui::run_selection_ui(screenshot.clone())?;

    match selection_result {
        Some((rect, ui_size)) => {
            // Processing
            let base64_img = ImageProcessor::process_selection(&screenshot, rect, ui_size)
                .context("Failed to process selection")?;

            // If prompt was empty, ask now
            let mut prompt_text = args.prompt.join(" ");
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
            println!(); // Spacer
            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                    .template("{spinner:.green} {msg}")?
            );
            spinner.set_message(format!("Analyzing with {}...", config.model_name));
            spinner.enable_steady_tick(Duration::from_millis(100));

            let client = GeminiClient::new(&config);
            let response_result = client?.analyze_image(base64_img, prompt_text).await;

            spinner.finish_and_clear();

            match response_result {
                Ok(response) => {
                    // Render Markdown
                    print_markdown(&response);

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

/// Helper to print markdown
fn print_markdown(text: &str) {
    let mut skin = MadSkin::default();
    skin.bold.set_fg(Color::Yellow);
    skin.italic.set_fg(Color::Magenta);
    skin.code_block.set_bg(Color::Rgb { r: 40, g: 40, b: 40} );

    skin.print_text(text);
}