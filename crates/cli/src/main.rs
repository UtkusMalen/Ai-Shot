use ai_shot_core::{
    capture::ScreenCapturer,
    config::Config,
    init,
    ui
};
use anyhow::{Context, Result};
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
    if let Err(e) = ui::run_selection_ui(screenshot, config) {
        eprintln!("UI Error: {}", e);
    }
    
    Ok(())
}