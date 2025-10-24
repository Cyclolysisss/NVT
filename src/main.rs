mod nvt_models;
mod nvt_views;
mod nvt_controllers;
mod nvt_gui;

use nvt_controllers::NVTControllers;
use clap::Parser;

/// TBM Next Vehicle - Real-time transit tracker for Bordeaux Métropole
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in CLI mode (terminal interface) instead of GUI mode
    #[arg(long, default_value_t = false)]
    cli: bool,
}

fn main() {
    let args = Args::parse();
    
    // Set up panic hook for better error messages
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("\n{}", "═".repeat(70));
        eprintln!("❌ APPLICATION PANIC");
        eprintln!("{}", "═".repeat(70));
        eprintln!("\nThe application encountered an unexpected error:");
        eprintln!("{}", panic_info);
        eprintln!("\n💡 Troubleshooting:");
        eprintln!("  • Please restart the application");
        eprintln!("  • Check your internet connection");
        eprintln!("  • Report this issue if it persists");
        eprintln!("\n{}", "═".repeat(70));
    }));

    if args.cli {
        // Run CLI mode
        match std::panic::catch_unwind(|| {
            NVTControllers::run();
        }) {
            Ok(_) => {
                // Normal exit
            }
            Err(_) => {
                eprintln!("\n⚠️  Application terminated unexpectedly");
                std::process::exit(1);
            }
        }
    } else {
        // Run GUI mode
        if let Err(e) = nvt_gui::run_gui() {
            eprintln!("Failed to start GUI: {}", e);
            std::process::exit(1);
        }
    }
}