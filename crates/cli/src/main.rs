//! AI-Shot CLI Application
//!
//! A command-line tool for capturing screenshots and analyzing them with
//! Google's Gemini AI.

use ai_shot_core::{init, AiShot, Config};
use anyhow::{Context, Result};
use clap::Parser;
use std::process::Command;

/// AI-powered screenshot analysis tool using Google Gemini.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Prompt to send to Gemini (optional, uses default if empty)
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,

    /// Override the model defined in .env
    #[arg(short, long)]
    model: Option<String>,

    /// Copy the result to clipboard automatically
    #[arg(short, long, default_value_t = false)]
    copy: bool,

    /// Select which monitor to capture (0-indexed)
    #[arg(long, default_value_t = 0)]
    monitor: usize,

    /// List available monitors and exit
    #[arg(long)]
    list_monitors: bool,

    /// Run in background mode, listening for Ctrl+Alt+X hotkey
    #[arg(long)]
    daemon: bool,

    /// Load image from path instead of capturing (internal use)
    #[arg(long)]
    image_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment and parse arguments
    init();
    let args = Args::parse();

    // Handle daemon mode separately (blocking operation)
    if args.daemon {
        return run_daemon();
    }

    // Build configuration, applying CLI overrides
    let config = build_config(&args)?;

    // Create the application instance
    let app = AiShot::with_config(config).context("Failed to initialize ai-shot")?;

    // Handle --image-path (Internal fast-path)
    if let Some(path) = args.image_path {
        let img = image::open(&path)
            .with_context(|| format!("Failed to load image from path: {}", path))?;
        app.run_interactive_with_image(img)?;
        return Ok(());
    }

    // Handle --list-monitors
    if args.list_monitors {
        println!("Available monitors:");
        for info in app.list_monitors() {
            println!("  {}", info);
        }
        return Ok(());
    }

    // Run the interactive selection UI
    app.run_interactive(args.monitor)
        .context("Failed to run interactive mode. Try --list-monitors to check available indices")?;

    Ok(())
}

/// Builds configuration from environment with CLI overrides.
fn build_config(args: &Args) -> Result<Config> {
    let mut builder = Config::builder();

    if let Some(ref model) = args.model {
        builder = builder.with_model(model);
    }

    builder.build().context(
        "Failed to load configuration.",
    )
}

/// Runs the background daemon that listens for the Ctrl+Alt+X hotkey.
fn run_daemon() -> Result<()> {
    use rdev::{listen, EventType, Key};
    use std::sync::Arc;

    println!("AI-Shot Daemon Started");
    println!("   Press Ctrl+Alt+X to capture a screenshot");
    println!("   Press Ctrl+C to exit");

    // Initialize core once to warm up screens
    let app = Arc::new(AiShot::new().context("Failed to initialize daemon context")?);
    
    let mut ctrl_pressed = false;
    let mut alt_pressed = false;

    // Listen for global keyboard events
    let listen_result = listen(move |event| {
        match event.event_type {
            EventType::KeyPress(key) => {
                match key {
                    Key::ControlLeft | Key::ControlRight => ctrl_pressed = true,
                    Key::Alt | Key::AltGr => alt_pressed = true,
                    Key::KeyX => {
                        if ctrl_pressed && alt_pressed {
                            capture_and_spawn(app.clone());
                        }
                    }
                    _ => {}
                }
            }
            EventType::KeyRelease(key) => {
                match key {
                    Key::ControlLeft | Key::ControlRight => ctrl_pressed = false,
                    Key::Alt | Key::AltGr => alt_pressed = false,
                    _ => {}
                }
            }
            _ => {}
        }
    });

    if let Err(error) = listen_result {
        anyhow::bail!("Failed to start keyboard listener: {:?}", error);
    }

    Ok(())
}

/// Captures the screen immediately and spawns the UI process.
fn capture_and_spawn(app: std::sync::Arc<AiShot>) {
    println!("Hotkey triggered! Capturing...");
    
    // Capture immediately in this process (fast, no startup overhead)
    // We capture the primary monitor (0) for now.
    match app.capture(0) {
        Ok(screenshot) => {
            // Save to temporary file
            let temp_path = std::env::temp_dir().join("ai_shot_rapid_capture.png");
            match screenshot.save(&temp_path) {
                Ok(_) => {
                    spawn_process_with_image(&temp_path);
                }
                Err(e) => eprintln!("❌ Failed to save temp image: {}", e),
            }
        }
        Err(e) => eprintln!("❌ Failed to capture screen: {}", e),
    }
}

/// Spawns the main process processing the saved image
fn spawn_process_with_image(path: &std::path::Path) {
    match std::env::current_exe() {
        Ok(exe_path) => {
            if let Err(e) = Command::new(exe_path)
                .arg("--image-path")
                .arg(path)
                .spawn() 
            {
                eprintln!("❌ Failed to spawn UI process: {}", e);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to get executable path: {}", e);
        }
    }
}