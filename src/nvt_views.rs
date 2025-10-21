// Views for TBM Next Vehicle application
use crate::nvt_models::{Line, Stop, RealTimeInfo, NetworkData, NVTModels};
use crate::nvt_controllers::NVTControllers;
use std::io::{self, Write};

pub struct NVTViews;

impl NVTViews {
    /// Show main menu with better formatting
    pub fn show_menu() {
        println!("\n{}", "═".repeat(60));
        println!("     🚊 TBM NEXT VEHICLE - BORDEAUX MÉTROPOLE");
        println!("{}", "═".repeat(60));
        println!("\n📋 MENU OPTIONS");
        println!("  1️⃣  Select a line");
        println!("  2️⃣  Select a stop");
        println!("  3️⃣  Show next vehicles in real-time 🔄");
        println!("  4️⃣  Browse all stops");
        println!("  5️⃣  Browse all lines");
        println!("  6️⃣  Show cache statistics 📊");
        println!("  0️⃣  Quit application");
        println!("\n{}", "─".repeat(60));
        print!("➜ Your choice: ");
        let _ = io::stdout().flush();
    }

    /// Prompt for line input with examples
    pub fn prompt_line() -> String {
        print!("\n🚌 Enter line name or code\n");
        print!("   Examples: 'A', 'C', '1', '23', 'Tram A'\n");
        print!("➜ Line: ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        input.trim().to_string()
    }

    /// Prompt for stop input with examples
    pub fn prompt_stop() -> String {
        print!("\n📍 Enter stop name\n");
        print!("   Examples: 'Quinconces', 'Victoire', 'Gare Saint-Jean'\n");
        print!("➜ Stop: ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        input.trim().to_string()
    }

    /// Show selected line with better formatting
    pub fn show_line_selected(line: &Line) {
        println!("\n{}", "─".repeat(60));
        println!("✓ Line selected: {} - {}",
                 Self::colorize_line(&line.line_code, &line.color),
                 line.line_name
        );

        if !line.destinations.is_empty() {
            println!("\n  🎯 Destinations:");
            for (dir_ref, place_name) in &line.destinations {
                let direction = if dir_ref == "0" { "→ Outbound" } else { "← Inbound" };
                println!("     {} : {}", direction, place_name);
            }
        }

        if !line.alerts.is_empty() {
            println!("\n  ⚠️  Alerts (Active or Future):");
            for alert in &line.alerts {
                println!("     • {}", alert.text);
            }
        }

        println!("{}", "─".repeat(60));
    }

    /// Show selected stop with comprehensive info
    pub fn show_stop_selected(stop: &Stop, network: &NetworkData) {
        println!("\n{}", "─".repeat(60));
        println!("✓ Stop selected: {}", stop.stop_name);
        println!("  📌 Location: ({:.6}, {:.6})", stop.latitude, stop.longitude);
        println!("  🆔 Stop ID: {}", stop.stop_id);

        if !stop.lines.is_empty() {
            println!("\n  🚌 Lines serving this stop ({}):", stop.lines.len());
            let mut line_display = Vec::new();
            for line_ref in &stop.lines {
                if let Some(line) = network.lines.iter().find(|l| &l.line_ref == line_ref) {
                    line_display.push(format!("{}",
                                              Self::colorize_line(&line.line_code, &line.color)
                    ));
                }
            }
            // Display lines in rows of 10
            for chunk in line_display.chunks(10) {
                println!("     {}", chunk.join(" "));
            }
        }

        if !stop.alerts.is_empty() {
            println!("\n  ⚠️  Alerts: (Active or Future)");
            for alert in &stop.alerts {
                println!("     • {}", alert.text);
            }
        }

        println!("{}", "─".repeat(60));
    }

    /// Show stop choices when multiple matches
    /// Show stop choices when multiple matches
    pub fn show_stop_choices(stops: &[&Stop], network: &NetworkData) {
        println!("\n📍 Multiple stops found. Please choose:");
        println!("{}", "─".repeat(60));
        for (i, stop) in stops.iter().enumerate() {
            println!("  {}. {} (ID: {})", i + 1, stop.stop_name, stop.stop_id);
            println!("     📌 ({:.6}, {:.6})", stop.latitude, stop.longitude);

            // Add lines information
            if !stop.lines.is_empty() {
                let line_codes: Vec<String> = stop.lines.iter()
                    .filter_map(|line_ref| {
                        network.lines.iter()
                            .find(|l| &l.line_ref == line_ref)
                            .map(|l| Self::colorize_line(&l.line_code, &l.color))
                    })
                    .take(10)
                    .collect();

                print!("     🚌 Lines: {}", line_codes.join(" "));
                if stop.lines.len() > 10 {
                    print!(" (+{} more)", stop.lines.len() - 10);
                }
                println!();
            }

            if i < stops.len() - 1 {
                println!();
            }
        }
        println!("{}", "─".repeat(60));
    }
    /// Show line suggestions with better formatting
    pub fn show_line_suggestions(lines: &[&Line]) {
        println!("\n💡 Did you mean one of these lines?");
        println!("{}", "─".repeat(60));
        for line in lines {
            println!("  • {} {} - {}",
                     Self::colorize_line(&line.line_code, &line.color),
                     line.line_name,
                     line.line_ref
            );
        }
        println!("{}", "─".repeat(60));
    }

    /// Show next vehicles for a stop with improved display
    pub fn show_next_vehicles(
        stop: &Stop,
        vehicles: &[&RealTimeInfo],
        selected_line: Option<&Line>,
        network: &NetworkData,
    ) {
        println!("\n{}", "═".repeat(70));
        println!("🕐 NEXT VEHICLES AT: {}", stop.stop_name);
        if let Some(line) = selected_line {
            println!("   Filtered by line: {} {}",
                     Self::colorize_line(&line.line_code, &line.color),
                     line.line_name
            );
        }
        println!("{}", "═".repeat(70));

        if vehicles.is_empty() {
            Self::show_no_vehicles_message(stop, selected_line);
            return;
        }

        let now = chrono::Utc::now().timestamp();
        let is_all_scheduled = vehicles.iter().all(|v| NVTControllers::is_scheduled(v));

        if is_all_scheduled {
            println!("\n📅 Showing scheduled times (real-time tracking unavailable)");
        } else {
            println!("\n📡 Showing real-time vehicle positions");
        }

        println!("{}", "─".repeat(70));

        let max_display = 10;
        for (i, rt) in vehicles.iter().take(max_display).enumerate() {
            Self::display_vehicle_info(i + 1, rt, network, now);
            if i < vehicles.len().min(max_display) - 1 {
                println!("{}", "  ┄".repeat(35));
            }
        }

        if vehicles.len() > max_display {
            println!("\n  ... and {} more upcoming vehicles", vehicles.len() - max_display);
        }

        // Show alerts if any
        if !stop.alerts.is_empty() {
            println!("\n{}", "═".repeat(70));
            println!("⚠️  ALERTS (ACTIVE OR FUTURE) FOR THIS STOP:");
            for alert in &stop.alerts {
                println!("  • {}", alert.text);
            }
        }

        println!("{}", "═".repeat(70));
    }

    /// Display individual vehicle information
    fn display_vehicle_info(
        index: usize,
        rt: &RealTimeInfo,
        network: &NetworkData,
        now: i64,
    ) {
        // Find the line for this vehicle
        let line = rt.route_id.as_ref().and_then(|route_id| {
            network.lines.iter().find(|l| {
                NVTModels::extract_line_id(&l.line_ref) == Some(route_id.as_str())
            })
        });

        println!("\n  {}. {}", index, if let Some(l) = line {
            format!("{} {}",
                    Self::colorize_line(&l.line_code, &l.color),
                    l.line_name
            )
        } else {
            format!("Line (Trip: {})", &rt.trip_id[..rt.trip_id.len().min(8)])
        });

        // Show destination
        if let Some(destination) = &rt.destination {
            println!("     🎯 Direction: {}", destination);
        } else if let (Some(l), Some(dir_id)) = (line, rt.direction_id) {
            if let Some((_, dest)) = l.destinations.iter()
                .find(|(d, _)| d == &dir_id.to_string()) {
                println!("     🎯 Direction: {}", dest);
            }
        }

        // Show timing information
        if let Some(ts) = rt.timestamp {
            let time_str = NVTModels::format_timestamp(ts);
            let minutes = NVTControllers::minutes_until_arrival(ts, now);

            print!("     ⏰ ");
            if minutes < 0 {
                println!("Time: {} (⚫ departed)", time_str);
            } else if minutes == 0 {
                println!("Time: {} (🔴 ARRIVING NOW!)", time_str);
            } else if minutes <= 2 {
                println!("Time: {} (🔴 {} min - approaching)", time_str, minutes);
            } else if minutes <= 5 {
                println!("Time: {} (🟡 {} min)", time_str, minutes);
            } else if minutes <= 15 {
                println!("Time: {} (🟢 {} min)", time_str, minutes);
            } else {
                println!("Time: {} ({} min)", time_str, minutes);
            }
        } else {
            println!("     ⏰ Time: Not available");
        }

        // Show delay if available
        if let Some(delay) = rt.delay {
            let delay_str = NVTControllers::format_delay(delay);
            print!("     ⏱️  Status: ");
            if delay > 180 {
                println!("🔴 {} (significant delay)", delay_str);
            } else if delay > 60 {
                println!("🟡 {}", delay_str);
            } else if delay < -60 {
                println!("🟢 {} (ahead of schedule)", delay_str);
            } else {
                println!("🟢 {}", delay_str);
            }
        }

        // Show data source
        if NVTControllers::is_scheduled(rt) {
            println!("     📊 Source: Scheduled timetable");
        } else {
            println!("     📊 Source: Real-time GPS tracking");
            if rt.vehicle_id != "Unknown" {
                println!("     🚌 Vehicle ID: {}", rt.vehicle_id);
            }
            if rt.latitude != 0.0 && rt.longitude != 0.0 {
                println!("     📍 Position: ({:.4}, {:.4})", rt.latitude, rt.longitude);
            }
        }
    }

    /// Show message when no vehicles are found
    fn show_no_vehicles_message(stop: &Stop, selected_line: Option<&Line>) {
        println!("\n⚠️  No upcoming vehicles found");
        println!("\n📋 Possible reasons:");

        if selected_line.is_some() {
            println!("  • No vehicles on the selected line are currently approaching this stop");
            println!("  • Try viewing all lines at this stop (option 3 without line filter)");
        } else {
            println!("  • Service may not be operating at this time");
            println!("  • This stop might have limited service");
            println!("  • Real-time data temporarily unavailable");
        }

        println!("\n💡 Suggestions:");
        println!("  • Check the stop name is correct (option 2)");
        println!("  • Try again in a few moments");
        println!("  • Visit https://www.infotbm.com/ for service status");

        println!("\n📍 Stop Information:");
        println!("  Name: {}", stop.stop_name);
        println!("  ID: {}", stop.stop_id);
        println!("  Lines serving this stop: {}", stop.lines.len());
    }

    /// Show all stops with improved pagination
    pub fn show_all_stops(stops: &[Stop], network: &NetworkData) {
        println!("\n{}", "═".repeat(70));
        println!("📍 ALL STOPS IN TBM NETWORK ({} total)", stops.len());
        println!("{}", "═".repeat(70));

        const PAGE_SIZE: usize = 20;
        let total_pages = (stops.len() + PAGE_SIZE - 1) / PAGE_SIZE;

        for page in 0..total_pages {
            let start = page * PAGE_SIZE;
            let end = std::cmp::min(start + PAGE_SIZE, stops.len());

            println!("\n📄 Page {} of {} (stops {} - {})",
                     page + 1, total_pages, start + 1, end);
            println!("{}", "─".repeat(70));

            for (idx, stop) in stops[start..end].iter().enumerate() {
                println!("\n  {}. {} (ID: {})",
                         start + idx + 1, stop.stop_name, stop.stop_id);
                println!("     📌 Location: ({:.6}, {:.6})",
                         stop.latitude, stop.longitude);

                if !stop.lines.is_empty() {
                    let line_codes: Vec<String> = stop.lines.iter()
                        .filter_map(|line_ref| {
                            network.lines.iter()
                                .find(|l| &l.line_ref == line_ref)
                                .map(|l| Self::colorize_line(&l.line_code, &l.color))
                        })
                        .take(15)
                        .collect();

                    print!("     🚌 Lines: {}", line_codes.join(" "));
                    if stop.lines.len() > 15 {
                        print!(" (+{} more)", stop.lines.len() - 15);
                    }
                    println!();
                }
            }

            if page < total_pages - 1 {
                println!("\n{}", "─".repeat(70));
                print!("Press Enter for next page (or Ctrl+C to cancel)...");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
            }
        }

        println!("\n{}", "═".repeat(70));
        println!("✓ End of stops list");
    }

    /// Show all lines with better organization
    pub fn show_all_lines(lines: &[Line]) {
        println!("\n{}", "═".repeat(70));
        println!("🚌 ALL LINES IN TBM NETWORK ({} total)", lines.len());
        println!("{}", "═".repeat(70));

        // Group lines by type (Tram, Bus, etc.)
        let mut trams: Vec<&Line> = Vec::new();
        let mut buses: Vec<&Line> = Vec::new();

        for line in lines {
            if line.line_code.len() == 1 && line.line_code.chars().all(|c| c.is_alphabetic()) {
                trams.push(line);
            } else {
                buses.push(line);
            }
        }

        if !trams.is_empty() {
            println!("\n🚊 TRAM/BRT LINES ({}):", trams.len());
            println!("{}", "─".repeat(70));
            for line in trams {
                Self::display_line_info(line);
            }
        }

        if !buses.is_empty() {
            println!("\n🚌 BUS LINES ({}):", buses.len());
            println!("{}", "─".repeat(70));
            for (idx, line) in buses.iter().enumerate() {
                Self::display_line_info(line);
                if (idx + 1) % 10 == 0 && idx < buses.len() - 1 {
                    println!("\n{}", "  ┄".repeat(35));
                }
            }
        }

        println!("\n{}", "═".repeat(70));
    }

    /// Display individual line information
    fn display_line_info(line: &Line) {
        println!("\n  {} {} - {}",
                 Self::colorize_line(&line.line_code, &line.color),
                 line.line_name,
                 line.line_ref
        );

        if !line.destinations.is_empty() {
            for (dir_ref, place_name) in &line.destinations {
                let arrow = if dir_ref == "0" { "  →" } else { "  ←" };
                println!("    {} {}", arrow, place_name);
            }
        }
        if !line.alerts.is_empty() {
            println!("    ⚠️  {} Alert(s) (active or future)", line.alerts.len());
        }
    }

    /// Error messages with helpful context
    pub fn invalid_line(input: &str) {
        println!("\n{}", "─".repeat(60));
        println!("✗ Line '{}' not found", input);
        println!("\n💡 Tips:");
        println!("  • Check the spelling");
        println!("  • Try using just the line code (e.g., 'A', '1', '23')");
        println!("  • Use option 5 to browse all available lines");
        println!("{}", "─".repeat(60));
    }

    pub fn invalid_stop(input: &str) {
        println!("\n{}", "─".repeat(60));
        println!("✗ Stop '{}' not found", input);
        println!("\n💡 Tips:");
        println!("  • Try a partial name (e.g., 'Quin' for 'Quinconces')");
        println!("  • Check the spelling");
        println!("  • Use option 4 to browse all available stops");
        println!("{}", "─".repeat(60));
    }

    pub fn invalid_stop_for_line(line_name: &str) {
        println!("\n{}", "─".repeat(60));
        println!("✗ This stop is not served by line '{}'", line_name);
        println!("\n💡 Suggestions:");
        println!("  • Clear line selection and try again");
        println!("  • Check if you selected the correct stop");
        println!("  • Use option 2 to see which lines serve a stop");
        println!("{}", "─".repeat(60));
    }

    pub fn no_line_selected() {
        println!("\n{}", "─".repeat(60));
        println!("ℹ️  No line currently selected");
        println!("   Showing all lines at the stop");
        println!("{}", "─".repeat(60));
    }

    pub fn no_stop_selected() {
        println!("\n{}", "─".repeat(60));
        println!("✗ No stop selected");
        println!("\n💡 Please select a stop first:");
        println!("  • Use option 2 to select a stop");
        println!("  • Or use option 4 to browse all stops");
        println!("{}", "─".repeat(60));
    }

    /// Warning messages
    pub fn all_stops_warning() {
        println!("\n{}", "─".repeat(60));
        println!("⚠️  WARNING: Large Data Display");
        println!("\n   This will display ALL stops in the TBM network.");
        println!("   • This may take some time to load");
        println!("   • Results will be paginated for easier viewing");
        println!("{}", "─".repeat(60));
    }

    pub fn all_lines_warning() {
        println!("\n{}", "─".repeat(60));
        println!("⚠️  INFO: Complete Line List");
        println!("\n   This will display ALL lines in the TBM network.");
        println!("   Lines will be organized by type (Trams, Buses)");
        println!("{}", "─".repeat(60));
    }

    /// Network error message
    pub fn network_error(error: &str) {
        println!("\n{}", "═".repeat(60));
        println!("❌ NETWORK ERROR");
        println!("{}", "═".repeat(60));
        println!("\n{}", error);
        println!("\n💡 Troubleshooting:");
        println!("  • Check your internet connection");
        println!("  • The TBM API might be temporarily unavailable");
        println!("  • Try again in a few moments");
        println!("  • Visit https://www.infotbm.com/ for service status");
        println!("\n{}", "═".repeat(60));
    }

    /// Loading indicator
    pub fn show_loading(message: &str) {
        print!("\r🔄 {}...", message);
        io::stdout().flush().unwrap();
    }

    pub fn clear_loading() {
        print!("\r{}\r", " ".repeat(60));
        io::stdout().flush().unwrap();
    }

    /// Success messages
    pub fn operation_cancelled() {
        println!("\n✓ Operation cancelled");
    }

    pub fn goodbye_message() {
        println!("\n{}", "═".repeat(60));
        println!("       👋 Thank you for using TBM Next Vehicle!");
        println!("           Visit us again for real-time updates");
        println!("{}", "═".repeat(60));
        println!();
    }

    /// Colorize line code with ANSI colors (improved contrast)
    fn colorize_line(code: &str, hex_color: &str) -> String {
        let (r, g, b) = NVTModels::parse_hex_color(hex_color);

        // Calculate relative luminance for contrast
        let luminance = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0;

        // Use white text on dark backgrounds, black on light backgrounds
        let text_color = if luminance > 0.5 { "30" } else { "97" };

        // Format with background color and contrasting text
        format!(
            "\x1b[48;2;{};{};{}m\x1b[{}m {} \x1b[0m",
            r, g, b, text_color, code
        )
    }

    /// Display a progress bar for long operations
    pub fn show_progress(current: usize, total: usize, label: &str) {
        let percentage = (current as f32 / total as f32 * 100.0) as usize;
        let bar_length = 40;
        let filled = (bar_length * current) / total;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_length - filled);

        print!("\r{}: [{}] {}% ({}/{})", label, bar, percentage, current, total);
        io::stdout().flush().unwrap();

        if current == total {
            println!();
        }
    }
}