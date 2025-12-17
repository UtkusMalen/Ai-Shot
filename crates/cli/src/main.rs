use ai_shot_core::{
    capture::ScreenCapturer,
    config::Config,
    init,
    ui
};
use anyhow::{Context, Result};
use clap::Parser;
use std::process::Command;

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

    /// Run in background mode, listening for Ctrl+Alt+X
    #[arg(long)]
    daemon: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup
    let _ = dotenvy::dotenv();
    init();
    let args = Args::parse();

    if args.daemon {
        println!("Starting Daemon Mode (Ctrl+Alt+X to capture)");
        use rdev::{listen, EventType, Key};
        // rdev callback is FnMut, but state needs to persist.
        // wait, listen takes a callback.
        // We can capture mut variables in closure?
        // listen signature: `pub fn listen<F>(callback: F) -> Result<(), ListenError> where F: FnMut(Event) + 'static` (on some platforms) or just `Fn(Event)`.
        // If it's Fn, we need interior mutability. If FnMut, we are good.
        // rdev listen callback is usually `FnMut`.
        
        let mut ctrl = false;
        let mut alt = false;
        
        if let Err(error) = listen(move |event| {
            match event.event_type {
                EventType::KeyPress(key) => {
                    match key {
                        Key::ControlLeft | Key::ControlRight => ctrl = true,
                        Key::Alt | Key::AltGr => alt = true,
                        Key::KeyX => {
                            if ctrl && alt {
                                println!("Hotkey triggered! Launching capture...");
                                if let Ok(exe) = std::env::current_exe() {
                                    if let Err(e) = Command::new(exe).spawn() {
                                        eprintln!("Failed to spawn child: {}", e);
                                    }
                                }
                            }
                        },
                        _ => {}
                    }
                },
                EventType::KeyRelease(key) => {
                    match key {
                        Key::ControlLeft | Key::ControlRight => ctrl = false,
                        Key::Alt | Key::AltGr => alt = false,
                        _ => {}
                    }
                },
                _ => {}
            }
        }) {
            eprintln!("Error: {:?}", error);
        }
        return Ok(());
    }

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