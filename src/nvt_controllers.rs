// Controllers for TBM Next Vehicle application
use crate::nvt_models::{NVTModels, NetworkData, CachedNetworkData, Line, Stop, RealTimeInfo};
use crate::nvt_views::NVTViews;
use std::io::{self, Write};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct NVTControllers;

impl NVTControllers {
    /// Main application loop
    pub fn run() {
        Self::show_welcome_screen();

        println!("\n🔄 Loading TBM network data...");
        println!("   Please wait, this may take a moment...");

        // Initialize cache
        let mut cache = match NVTModels::initialize_cache() {
            Ok(data) => {
                println!("\n✓ Network data loaded successfully!");
                data
            }
            Err(e) => {
                NVTViews::network_error(&format!("{}", e));
                println!("\n💡 Please ensure you have internet access and try again.");
                Self::pause();
                return;
            }
        };

        let mut selected_line: Option<String> = None;
        let mut selected_stop: Option<String> = None;

        loop {
            let network = cache.to_network_data();
            NVTViews::show_menu();

            let choice = Self::read_input();

            match choice.trim() {
                "1" => {
                    match Self::handle_line_selection(&network) {
                        Some(line_ref) => {
                            selected_line = Some(line_ref);
                            selected_stop = None; // Reset stop when changing line
                        }
                        None => {}
                    }
                    Self::pause();
                }
                "2" => {
                    selected_stop = Self::handle_stop_selection(&network, &selected_line);
                    Self::pause();
                }
                "3" => {
                    Self::handle_show_next_vehicle_with_refresh(
                        &mut cache,
                        &selected_line,
                        &selected_stop
                    );
                }
                "4" => {
                    Self::handle_show_all_stops(&network);
                    Self::pause();
                }
                "5" => {
                    Self::handle_show_all_lines(&network);
                    Self::pause();
                }
                "6" => {
                    println!("\n{}", NVTModels::get_cache_stats(&cache));
                    Self::pause();
                }
                "0" => {
                    NVTViews::goodbye_message();
                    break;
                }
                "" => {
                    // Just pressed Enter, show menu again
                }
                _ => {
                    println!("\n✗ Invalid option '{}'. Please select 0-6.", choice.trim());
                    Self::pause();
                }
            }
        }
    }

    /// Show welcome screen
    fn show_welcome_screen() {
        println!("\n{}", "═".repeat(70));
        println!("  ╔═══════════════════════════════════════════════════════════╗");
        println!("  ║         🚊 TBM NEXT VEHICLE - BORDEAUX MÉTROPOLE         ║");
        println!("  ║                  Real-Time Transit Tracker                ║");
        println!("  ╚═══════════════════════════════════════════════════════════╝");
        println!("{}", "═".repeat(70));
        println!("\n  📡 Features:");
        println!("     • Real-time vehicle positions and arrivals");
        println!("     • Complete stop and line information");
        println!("     • Service alerts and disruptions");
        println!("     • Auto-refreshing displays");
        println!("\n  🌐 Data source: TBM Open Data API");
        println!("     https://www.infotbm.com/");
        println!("\n{}", "═".repeat(70));
    }

    /// Simple pause - wait for Enter key
    fn pause() {
        print!("\n📌 Press Enter to continue...");
        io::stdout().flush().unwrap();
        let mut dummy = String::new();
        io::stdin().read_line(&mut dummy).unwrap();
    }

    /// Handle line selection with improved error handling
    fn handle_line_selection(network: &NetworkData) -> Option<String> {
        let line_input = NVTViews::prompt_line();

        if line_input.is_empty() {
            println!("\n⚠️  No input provided");
            return None;
        }

        let line = network.lines.iter().find(|l| {
            l.line_code.eq_ignore_ascii_case(&line_input) ||
                l.line_name.eq_ignore_ascii_case(&line_input)
        });

        match line {
            Some(l) => {
                NVTViews::show_line_selected(l);
                Some(l.line_ref.clone())
            }
            None => {
                NVTViews::invalid_line(&line_input);

                // Show suggestions
                let suggestions: Vec<&Line> = network.lines.iter()
                    .filter(|l| {
                        l.line_code.to_lowercase().contains(&line_input.to_lowercase()) ||
                            l.line_name.to_lowercase().contains(&line_input.to_lowercase())
                    })
                    .take(5)
                    .collect();

                if !suggestions.is_empty() {
                    NVTViews::show_line_suggestions(&suggestions);
                }

                None
            }
        }
    }

    /// Handle stop selection with improved matching
    fn handle_stop_selection(
        network: &NetworkData,
        selected_line: &Option<String>,
    ) -> Option<String> {
        let stop_input = NVTViews::prompt_stop();

        if stop_input.is_empty() {
            println!("\n⚠️  No input provided");
            return None;
        }

        // Find matching stops (partial match)
        let matching_stops: Vec<&Stop> = network.stops.iter()
            .filter(|s| s.stop_name.to_lowercase().contains(&stop_input.to_lowercase()))
            .collect();

        if matching_stops.is_empty() {
            NVTViews::invalid_stop(&stop_input);
            return None;
        }

        // Filter by selected line if applicable
        let filtered_stops: Vec<&Stop> = if let Some(line_ref) = selected_line {
            let filtered: Vec<&Stop> = matching_stops.into_iter()
                .filter(|s| s.lines.contains(line_ref))
                .collect();

            if filtered.is_empty() {
                let line_name = network.lines.iter()
                    .find(|l| &l.line_ref == line_ref)
                    .map(|l| l.line_name.as_str())
                    .unwrap_or("selected line");
                NVTViews::invalid_stop_for_line(line_name);
                return None;
            }
            filtered
        } else {
            matching_stops
        };

        // Handle selection
        let selected_stop = if filtered_stops.len() > 1 {
            NVTViews::show_stop_choices(&filtered_stops);
            Self::select_from_list(&filtered_stops)
        } else {
            Some(filtered_stops[0])
        };

        match selected_stop {
            Some(stop) => {
                NVTViews::show_stop_selected(stop, network);
                Some(stop.stop_id.clone())
            }
            None => None,
        }
    }

    /// Handle showing next vehicles with auto-refresh
    fn handle_show_next_vehicle_with_refresh(
        cache: &mut CachedNetworkData,
        selected_line: &Option<String>,
        selected_stop: &Option<String>,
    ) {
        if selected_stop.is_none() {
            NVTViews::no_stop_selected();
            Self::pause();
            return;
        }

        let stop_id = selected_stop.as_ref().unwrap().clone();
        let line_ref = selected_line.clone();

        println!("\n{}", "═".repeat(70));
        println!("🔄 AUTO-REFRESH MODE");
        println!("{}", "═".repeat(70));
        println!("   Data refreshes automatically every 30 seconds");
        println!("   Press ENTER at any time to return to menu");
        println!("{}", "═".repeat(70));

        let mut refresh_count = 0;

        loop {
            refresh_count += 1;

            // Refresh data (skip on first iteration)
            if refresh_count > 1 {
                NVTViews::show_loading("Refreshing data");

                match NVTModels::smart_refresh(cache) {
                    Ok(_) => {
                        NVTViews::clear_loading();
                        println!("✓ Data refreshed successfully");
                    }
                    Err(e) => {
                        NVTViews::clear_loading();
                        eprintln!("⚠️  Refresh failed: {}", e);
                        println!("   Using cached data, will retry next cycle...");
                    }
                }
            }

            // Display data
            Self::clear_screen();
            Self::display_refresh_header(refresh_count, cache);

            let network = cache.to_network_data();
            Self::display_next_vehicles(&network, &line_ref, &Some(stop_id.clone()));

            // Show cache stats
            println!("\n{}", NVTModels::get_cache_stats(cache));

            // Wait for input or timeout
            println!("\n{}", "─".repeat(70));
            println!("⏱️  Next refresh in 30 seconds (or press ENTER to exit)");
            println!("{}", "─".repeat(70));

            if Self::wait_for_input_or_timeout(30) {
                println!("\n👋 Exiting auto-refresh mode...");
                // Don't call pause here - return directly
                return;
            }
        }
    }

    /// Wait for user input with timeout - COMPLETELY REWRITTEN
    fn wait_for_input_or_timeout(seconds: u64) -> bool {
        let exit_flag = Arc::new(Mutex::new(false));
        let exit_flag_clone = exit_flag.clone();

        // Spawn a thread that waits for Enter
        let handle = thread::spawn(move || {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let mut flag = exit_flag_clone.lock().unwrap();
                *flag = true;
            }
        });

        // Poll the flag with timeout
        let start = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(seconds);

        while start.elapsed() < timeout_duration {
            {
                let flag = exit_flag.lock().unwrap();
                if *flag {
                    // User pressed Enter - don't wait for thread
                    return true;
                }
            }
            // Sleep for a short time to avoid busy waiting
            thread::sleep(Duration::from_millis(100));
        }

        // Timeout reached - thread will be orphaned but that's ok
        // It will complete when user eventually presses Enter
        false
    }

    /// Display refresh header
    fn display_refresh_header(refresh_count: u32, cache: &CachedNetworkData) {
        let now = chrono::Utc::now();
        let paris_time = now.with_timezone(&chrono_tz::Europe::Paris);

        println!("\n{}", "═".repeat(70));
        println!("🔄 AUTO-REFRESH MODE - Update #{}", refresh_count);
        println!("📅 {}", paris_time.format("%A, %B %d, %Y at %H:%M:%S %Z"));
        println!("📊 {} vehicles tracked | ⚠️  {}  Alerts (active or future)",
                 cache.real_time.len(), cache.alerts.len());
        println!("{}", "═".repeat(70));
    }

    /// Display next vehicles (single display)
    fn display_next_vehicles(
        network: &NetworkData,
        selected_line: &Option<String>,
        selected_stop: &Option<String>,
    ) {
        if selected_stop.is_none() {
            NVTViews::no_stop_selected();
            return;
        }

        let stop_id = selected_stop.as_ref().unwrap();
        let stop = network.stops.iter().find(|s| &s.stop_id == stop_id);

        if stop.is_none() {
            println!("\n✗ Stop not found in network data");
            return;
        }

        let stop = stop.unwrap();
        let mut vehicles = NVTModels::get_next_vehicles_for_stop(&stop.stop_id, network);

        // Filter by line if selected
        if let Some(line_ref) = selected_line {
            let line = network.lines.iter().find(|l| &l.line_ref == line_ref);
            if let Some(line) = line {
                let line_id = NVTModels::extract_line_id(&line.line_ref).unwrap_or("");
                vehicles.retain(|v| {
                    v.route_id
                        .as_ref()
                        .map(|route_id| route_id == line_id)
                        .unwrap_or(false)
                });
            }
        }

        NVTViews::show_next_vehicles(
            stop,
            &vehicles,
            selected_line.as_ref().and_then(|lr| {
                network.lines.iter().find(|l| &l.line_ref == lr)
            }),
            network,
        );
    }

    /// Handle showing all stops
    fn handle_show_all_stops(network: &NetworkData) {
        NVTViews::all_stops_warning();
        print!("\nContinue? (y/n): ");
        io::stdout().flush().unwrap();

        let input = Self::read_input();
        if input.trim().eq_ignore_ascii_case("y") {
            NVTViews::show_all_stops(&network.stops, network);
        } else {
            NVTViews::operation_cancelled();
        }
    }

    /// Handle showing all lines
    fn handle_show_all_lines(network: &NetworkData) {
        NVTViews::all_lines_warning();
        print!("\nContinue? (y/n): ");
        io::stdout().flush().unwrap();

        let input = Self::read_input();
        if input.trim().eq_ignore_ascii_case("y") {
            NVTViews::show_all_lines(&network.lines);
        } else {
            NVTViews::operation_cancelled();
        }
    }

    /// Select from a list of items
    fn select_from_list<'a>(items: &[&'a Stop]) -> Option<&'a Stop> {
        print!("\n➜ Enter number (1-{}): ", items.len());
        io::stdout().flush().unwrap();

        let input = Self::read_input();

        match input.trim().parse::<usize>() {
            Ok(num) if num > 0 && num <= items.len() => Some(items[num - 1]),
            _ => {
                println!("✗ Invalid selection. Please enter a number between 1 and {}", items.len());
                None
            }
        }
    }

    /// Read input from stdin with error handling
    fn read_input() -> String {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => input,
            Err(e) => {
                eprintln!("⚠️  Error reading input: {}", e);
                String::new()
            }
        }
    }

    /// Clear screen (cross-platform)
    fn clear_screen() {
        // ANSI escape sequence to clear screen and move cursor to top-left
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Check if real-time info is from scheduled data
    pub fn is_scheduled(rt: &RealTimeInfo) -> bool {
        rt.vehicle_id == "scheduled" || rt.vehicle_id == "fallback_trip_update"
    }

    /// Calculate minutes until arrival
    pub fn minutes_until_arrival(timestamp: i64, now: i64) -> i64 {
        (timestamp - now) / 60
    }

    /// Format delay as a readable string
    pub fn format_delay(delay_seconds: i32) -> String {
        let minutes = delay_seconds / 60;
        let seconds = delay_seconds.abs() % 60;

        if delay_seconds >= -30 && delay_seconds <= 30 {
            "On time".to_string()
        } else if minutes == 0 {
            format!("{:+}s", delay_seconds)
        } else if seconds == 0 {
            format!("{:+} min", minutes)
        } else {
            format!("{:+} min {}s", minutes, seconds)
        }
    }

    /// Validate stop ID
    pub fn validate_stop_id(stop_id: &str, network: &NetworkData) -> bool {
        network.stops.iter().any(|s| s.stop_id == stop_id)
    }

    /// Validate line reference
    pub fn validate_line_ref(line_ref: &str, network: &NetworkData) -> bool {
        network.lines.iter().any(|l| l.line_ref == line_ref)
    }
}