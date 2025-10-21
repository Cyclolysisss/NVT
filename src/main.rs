mod nvt_models;
mod nvt_views;
mod nvt_controllers;

use nvt_controllers::NVTControllers;

fn main() {
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

    // Run the application
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
}