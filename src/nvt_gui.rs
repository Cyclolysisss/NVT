// GUI implementation for TBM Next Vehicle application using egui/eframe
use crate::nvt_models::{CachedNetworkData, Line, NetworkData, NVTModels, RealTimeInfo, Stop};
use chrono::{DateTime, Local};
use eframe::egui;
use egui::{Color32, RichText, Ui};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// Application State
// ============================================================================

#[derive(PartialEq, Clone, Copy)]
enum AppView {
    LineSelection,
    StopSelection,
    RealTimeArrivals,
    AllStopsBrowser,
    AllLinesBrowser,
    CacheStats,
}

pub struct NVTApp {
    // Network data cache
    cache: Arc<Mutex<Option<CachedNetworkData>>>,
    network: Arc<Mutex<Option<NetworkData>>>,
    
    // Loading states
    is_loading: bool,
    loading_message: String,
    error_message: Option<String>,
    
    // Selected items
    selected_line: Option<String>,
    selected_stop: Option<String>,
    
    // Current view
    current_view: AppView,
    
    // Search inputs
    line_search: String,
    stop_search: String,
    
    // Auto-refresh settings
    auto_refresh_enabled: bool,
    last_refresh: Option<SystemTime>,
    refresh_counter: usize,
    
    // Pagination for browsers
    stops_page: usize,
    stops_per_page: usize,
    
    // Background task for initialization
    init_promise: Option<poll_promise::Promise<Result<CachedNetworkData, String>>>,
}

impl Default for NVTApp {
    fn default() -> Self {
        Self {
            cache: Arc::new(Mutex::new(None)),
            network: Arc::new(Mutex::new(None)),
            is_loading: true,
            loading_message: "Initializing...".to_string(),
            error_message: None,
            selected_line: None,
            selected_stop: None,
            current_view: AppView::LineSelection,
            line_search: String::new(),
            stop_search: String::new(),
            auto_refresh_enabled: false,
            last_refresh: None,
            refresh_counter: 0,
            stops_page: 0,
            stops_per_page: 50,
            init_promise: None,
        }
    }
}

// ============================================================================
// GUI Implementation
// ============================================================================

impl NVTApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        
        // Start loading data in background
        app.start_initialization();
        
        app
    }
    
    fn start_initialization(&mut self) {
        let promise = poll_promise::Promise::spawn_thread("init", || {
            match NVTModels::initialize_cache() {
                Ok(cache) => Ok(cache),
                Err(e) => Err(format!("{}", e)),
            }
        });
        self.init_promise = Some(promise);
        self.is_loading = true;
        self.loading_message = "Loading TBM network data...".to_string();
    }
    
    fn check_initialization(&mut self) {
        if let Some(promise) = &self.init_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(cache) => {
                        let network = cache.to_network_data();
                        *self.network.lock().unwrap() = Some(network);
                        *self.cache.lock().unwrap() = Some(cache.clone());
                        self.is_loading = false;
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.is_loading = false;
                        self.error_message = Some(format!("Failed to load network data: {}", e));
                    }
                }
                self.init_promise = None;
            }
        }
    }
    
    fn refresh_dynamic_data(&mut self) {
        if let Some(cache) = self.cache.lock().unwrap().as_mut() {
            if cache.needs_dynamic_refresh(30) {
                match NVTModels::smart_refresh(cache) {
                    Ok(()) => {
                        let network = cache.to_network_data();
                        *self.network.lock().unwrap() = Some(network);
                        self.last_refresh = Some(SystemTime::now());
                        self.refresh_counter += 1;
                    }
                    Err(e) => {
                        eprintln!("Failed to refresh data: {}", e);
                    }
                }
            }
        }
    }
}

impl eframe::App for NVTApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if initialization is complete
        self.check_initialization();
        
        // Auto-refresh if enabled
        if self.auto_refresh_enabled && !self.is_loading {
            ctx.request_repaint_after(Duration::from_secs(1));
            if let Some(last) = self.last_refresh {
                if last.elapsed().unwrap_or(Duration::from_secs(0)) >= Duration::from_secs(30) {
                    self.refresh_dynamic_data();
                }
            } else {
                self.refresh_dynamic_data();
            }
        }
        
        // Top panel with header
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🚊 TBM Next Vehicle - Bordeaux Métropole");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let now: DateTime<Local> = Local::now();
                    ui.label(now.format("%H:%M:%S").to_string());
                });
            });
        });
        
        // Show loading screen or main UI
        if self.is_loading {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.label(&self.loading_message);
                    });
                });
            });
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        
        // Show error if any
        if let Some(error) = self.error_message.clone() {
            let mut should_retry = false;
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.colored_label(Color32::RED, "❌ Error");
                        ui.label(&error);
                        if ui.button("Retry").clicked() {
                            should_retry = true;
                        }
                    });
                });
            });
            if should_retry {
                self.start_initialization();
            }
            return;
        }
        
        // Left panel with navigation
        egui::SidePanel::left("nav_panel").min_width(200.0).show(ctx, |ui| {
            ui.heading("Navigation");
            ui.separator();
            
            if ui.selectable_label(self.current_view == AppView::LineSelection, "📍 Select Line").clicked() {
                self.current_view = AppView::LineSelection;
            }
            if ui.selectable_label(self.current_view == AppView::StopSelection, "🚏 Select Stop").clicked() {
                self.current_view = AppView::StopSelection;
            }
            if ui.selectable_label(self.current_view == AppView::RealTimeArrivals, "🔄 Real-Time Arrivals").clicked() {
                self.current_view = AppView::RealTimeArrivals;
            }
            ui.separator();
            if ui.selectable_label(self.current_view == AppView::AllStopsBrowser, "📋 All Stops").clicked() {
                self.current_view = AppView::AllStopsBrowser;
            }
            if ui.selectable_label(self.current_view == AppView::AllLinesBrowser, "🚌 All Lines").clicked() {
                self.current_view = AppView::AllLinesBrowser;
            }
            ui.separator();
            if ui.selectable_label(self.current_view == AppView::CacheStats, "📊 Cache Stats").clicked() {
                self.current_view = AppView::CacheStats;
            }
            
            ui.add_space(20.0);
            ui.separator();
            ui.label("Current Selection:");
            if let Some(line_ref) = &self.selected_line {
                ui.label(format!("Line: {}", line_ref));
            } else {
                ui.label("Line: None");
            }
            if let Some(stop_id) = &self.selected_stop {
                ui.label(format!("Stop: {}", stop_id));
            } else {
                ui.label("Stop: None");
            }
        });
        
        // Central panel with main content
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_view {
                AppView::LineSelection => self.show_line_selection(ui),
                AppView::StopSelection => self.show_stop_selection(ui),
                AppView::RealTimeArrivals => self.show_real_time_arrivals(ui),
                AppView::AllStopsBrowser => self.show_all_stops_browser(ui),
                AppView::AllLinesBrowser => self.show_all_lines_browser(ui),
                AppView::CacheStats => self.show_cache_stats(ui),
            }
        });
    }
}

// ============================================================================
// View Implementations
// ============================================================================

impl NVTApp {
    fn show_line_selection(&mut self, ui: &mut Ui) {
        ui.heading("Select a Line");
        ui.separator();
        
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.line_search);
            if ui.button("Clear").clicked() {
                self.line_search.clear();
            }
        });
        
        ui.separator();
        
        // Clone network data to avoid borrowing issues
        let network_opt = self.network.lock().unwrap().clone();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(network) = network_opt.as_ref() {
                let search_lower = self.line_search.to_lowercase();
                let filtered_lines: Vec<Line> = network.lines.iter()
                    .filter(|line| {
                        search_lower.is_empty() ||
                        line.line_code.to_lowercase().contains(&search_lower) ||
                        line.line_name.to_lowercase().contains(&search_lower)
                    })
                    .cloned()
                    .collect();
                
                if filtered_lines.is_empty() {
                    ui.label("No lines found matching your search.");
                } else {
                    for line in &filtered_lines {
                        self.show_line_card(ui, line);
                    }
                }
            }
        });
    }
    
    fn show_line_card(&mut self, ui: &mut Ui, line: &Line) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                // Line badge with color
                let color = parse_hex_color(&line.color);
                ui.colored_label(color, RichText::new(&line.line_code).size(18.0).strong());
                
                ui.vertical(|ui| {
                    ui.strong(&line.line_name);
                    if !line.destinations.is_empty() {
                        for (dir_ref, dest) in &line.destinations {
                            let arrow = if dir_ref == "0" { "→" } else { "←" };
                            ui.label(format!("{} {}", arrow, dest));
                        }
                    }
                    
                    if !line.alerts.is_empty() {
                        ui.colored_label(Color32::from_rgb(255, 165, 0), 
                            format!("⚠️ {} alert(s)", line.alerts.len()));
                    }
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Select").clicked() {
                        self.selected_line = Some(line.line_ref.clone());
                        self.selected_stop = None; // Reset stop when changing line
                    }
                });
            });
        });
        ui.add_space(5.0);
    }
    
    fn show_stop_selection(&mut self, ui: &mut Ui) {
        ui.heading("Select a Stop");
        ui.separator();
        
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.stop_search);
            if ui.button("Clear").clicked() {
                self.stop_search.clear();
            }
        });
        
        ui.separator();
        
        // Clone network data and selected line to avoid borrowing issues
        let network_opt = self.network.lock().unwrap().clone();
        let selected_line = self.selected_line.clone();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(network) = network_opt.as_ref() {
                let search_lower = self.stop_search.to_lowercase();
                let mut filtered_stops: Vec<Stop> = network.stops.iter()
                    .filter(|stop| {
                        search_lower.is_empty() ||
                        stop.stop_name.to_lowercase().contains(&search_lower) ||
                        stop.stop_id.to_lowercase().contains(&search_lower)
                    })
                    .cloned()
                    .collect();
                
                // If a line is selected, filter stops for that line
                if let Some(line_ref) = &selected_line {
                    filtered_stops.retain(|stop| stop.lines.contains(line_ref));
                }
                
                if filtered_stops.is_empty() {
                    if selected_line.is_some() {
                        ui.label("No stops found for the selected line matching your search.");
                    } else {
                        ui.label("No stops found matching your search.");
                    }
                } else {
                    for stop in &filtered_stops {
                        self.show_stop_card(ui, stop, &network);
                    }
                }
            }
        });
    }
    
    fn show_stop_card(&mut self, ui: &mut Ui, stop: &Stop, network: &NetworkData) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.strong(&stop.stop_name);
                    ui.label(format!("ID: {} | Lat: {:.6}, Lon: {:.6}", 
                        stop.stop_id, stop.latitude, stop.longitude));
                    
                    // Show lines serving this stop
                    if !stop.lines.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Lines:");
                            for line_ref in &stop.lines {
                                if let Some(line) = network.lines.iter().find(|l| &l.line_ref == line_ref) {
                                    let color = parse_hex_color(&line.color);
                                    ui.colored_label(color, &line.line_code);
                                }
                            }
                        });
                    }
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Select").clicked() {
                        self.selected_stop = Some(stop.stop_id.clone());
                    }
                });
            });
        });
        ui.add_space(5.0);
    }
    
    fn show_real_time_arrivals(&mut self, ui: &mut Ui) {
        ui.heading("Real-Time Arrivals");
        ui.separator();
        
        let mut should_refresh = false;
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.auto_refresh_enabled, "Auto-refresh (30s)");
            if ui.button("Refresh Now").clicked() {
                should_refresh = true;
            }
            if let Some(last) = self.last_refresh {
                let elapsed = last.elapsed().unwrap_or(Duration::from_secs(0));
                ui.label(format!("Last update: {}s ago", elapsed.as_secs()));
            }
        });
        
        if should_refresh {
            self.refresh_dynamic_data();
        }
        
        ui.separator();
        
        // Only require a stop to be selected (line is optional)
        if self.selected_stop.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Please select a stop to view real-time arrivals.");
                ui.label("Optionally select a line to filter results.");
            });
            return;
        }
        
        // Clone network data and selection to avoid borrowing issues
        let network_opt = self.network.lock().unwrap().clone();
        let line_ref_opt = self.selected_line.clone();
        let stop_id = self.selected_stop.clone().unwrap();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(network) = network_opt.as_ref() {
                // Find the stop
                let stop = network.stops.iter().find(|s| s.stop_id == stop_id);
                
                if let Some(stop) = stop {
                    // Display header based on whether a line is selected
                    if let Some(line_ref) = &line_ref_opt {
                        if let Some(line) = network.lines.iter().find(|l| &l.line_ref == line_ref) {
                            ui.strong(format!("Line {} at {}", line.line_code, stop.stop_name));
                        }
                    } else {
                        ui.strong(format!("All lines at {}", stop.stop_name));
                    }
                    ui.add_space(10.0);
                    
                    // Show alerts for the stop and selected line (if any)
                    let mut has_alerts = false;
                    if !stop.alerts.is_empty() {
                        has_alerts = true;
                    }
                    if let Some(line_ref) = &line_ref_opt {
                        if let Some(line) = network.lines.iter().find(|l| &l.line_ref == line_ref) {
                            if !line.alerts.is_empty() {
                                has_alerts = true;
                            }
                        }
                    }
                    
                    if has_alerts {
                        ui.group(|ui| {
                            ui.colored_label(Color32::from_rgb(255, 165, 0), "⚠️ Active Alerts");
                            for alert in &stop.alerts {
                                ui.label(format!("• {}", alert.text));
                            }
                            if let Some(line_ref) = &line_ref_opt {
                                if let Some(line) = network.lines.iter().find(|l| &l.line_ref == line_ref) {
                                    for alert in &line.alerts {
                                        if !alert.stop_ids.is_empty() && !alert.stop_ids.contains(&stop_id) {
                                            continue; // Skip alerts not relevant to this stop
                                        }
                                        ui.label(format!("• {}", alert.text));
                                    }
                                }
                            }
                        });
                        ui.add_space(10.0);
                    }
                    
                    // Get vehicles for all lines serving this stop, or just the selected line
                    let vehicles = if let Some(line_ref) = &line_ref_opt {
                        // Filter by selected line
                        Self::get_next_vehicles(&network, line_ref, &stop_id)
                    } else {
                        // Get vehicles for all lines serving this stop
                        Self::get_all_vehicles_at_stop(&network, &stop_id)
                    };
                    
                    if vehicles.is_empty() {
                        ui.label("No upcoming vehicles found.");
                        ui.label("This could mean:");
                        ui.label("• No vehicles are currently scheduled");
                        ui.label("• Service has ended for the day");
                        ui.label("• Real-time data is temporarily unavailable");
                    } else {
                        for (idx, (line_ref, vehicle, destination)) in vehicles.iter().enumerate() {
                            if let Some(line) = network.lines.iter().find(|l| &l.line_ref == line_ref) {
                                self.show_vehicle_card(ui, line, vehicle, destination, idx + 1);
                            }
                        }
                    }
                } else {
                    ui.label("Error: Selected stop not found.");
                }
            }
        });
    }
    
    fn show_vehicle_card(&self, ui: &mut Ui, line: &Line, vehicle: &RealTimeInfo, destination: &str, position: usize) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                // Position number
                ui.label(RichText::new(format!("{}.", position)).size(16.0).strong());
                
                // Line badge
                let color = parse_hex_color(&line.color);
                ui.colored_label(color, RichText::new(&line.line_code).size(16.0).strong());
                
                ui.vertical(|ui| {
                    ui.strong(format!("→ {}", destination));
                    
                    // Arrival time and countdown
                    if let Some(timestamp) = vehicle.timestamp {
                        let arrival_time = DateTime::from_timestamp(timestamp, 0)
                            .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
                        let now = chrono::Utc::now();
                        let diff = (arrival_time.timestamp() - now.timestamp()) / 60;
                        
                        let time_str = arrival_time.with_timezone(&chrono_tz::Europe::Paris)
                            .format("%H:%M:%S").to_string();
                        
                        let countdown_color = if diff <= 2 {
                            Color32::from_rgb(255, 0, 0) // Red for imminent
                        } else if diff <= 5 {
                            Color32::from_rgb(255, 165, 0) // Orange for soon
                        } else {
                            Color32::from_rgb(0, 200, 0) // Green for later
                        };
                        
                        let countdown_str = if diff <= 0 { "Now".to_string() } else { format!("{} min", diff) };
                        ui.label(format!("⏰ Time: {} ({})", time_str, countdown_str));
                        let status_str = if diff <= 0 { "Arriving".to_string() } else { format!("{} min", diff) };
                        ui.colored_label(countdown_color, format!("● {}", status_str));
                    }
                    
                    // Delay status
                    if let Some(delay) = vehicle.delay {
                        let delay_min = delay / 60;
                        let status_text = if delay_min > 2 {
                            format!("⏱️  Status: 🔴 Delayed by {} min", delay_min)
                        } else if delay_min < -2 {
                            format!("⏱️  Status: 🟢 Early by {} min", -delay_min)
                        } else {
                            "⏱️  Status: 🟢 On time".to_string()
                        };
                        ui.label(status_text);
                    }
                    
                    // Data source - check if we have GPS coordinates (real-time) or just timestamp (scheduled)
                    let has_gps = vehicle.latitude != 0.0 || vehicle.longitude != 0.0;
                    if has_gps && vehicle.timestamp.is_some() {
                        ui.label("📊 Source: Real-time GPS tracking");
                    } else if vehicle.timestamp.is_some() {
                        ui.label("📊 Source: Real-time prediction");
                    } else {
                        ui.label("📊 Source: Scheduled data");
                    }
                    
                    // Vehicle ID
                    if !vehicle.vehicle_id.is_empty() {
                        ui.label(format!("🚌 Vehicle ID: {}", vehicle.vehicle_id));
                    }
                });
            });
        });
        ui.add_space(5.0);
    }
    
    fn get_next_vehicles(network: &NetworkData, line_ref: &str, stop_id: &str) -> Vec<(String, RealTimeInfo, String)> {
        let mut vehicles: Vec<(String, RealTimeInfo, String)> = Vec::new();
        
        // Get vehicles from real-time data
        if let Some(line) = network.lines.iter().find(|l| l.line_ref == line_ref) {
            for vehicle in &line.real_time {
                if vehicle.stop_id.as_ref().map(|s| s.as_str()) == Some(stop_id) {
                    let dest = vehicle.destination.clone().unwrap_or_else(|| "Unknown".to_string());
                    vehicles.push((line_ref.to_string(), vehicle.clone(), dest));
                }
            }
        }
        
        // Sort by timestamp
        vehicles.sort_by_key(|(_, v, _)| v.timestamp.unwrap_or(i64::MAX));
        
        // Take next 5 vehicles
        vehicles.into_iter().take(5).collect()
    }
    
    fn get_all_vehicles_at_stop(network: &NetworkData, stop_id: &str) -> Vec<(String, RealTimeInfo, String)> {
        let mut vehicles: Vec<(String, RealTimeInfo, String)> = Vec::new();
        
        // Get the stop to find which lines serve it
        if let Some(stop) = network.stops.iter().find(|s| s.stop_id == stop_id) {
            // Iterate through all lines serving this stop
            for line_ref in &stop.lines {
                if let Some(line) = network.lines.iter().find(|l| &l.line_ref == line_ref) {
                    for vehicle in &line.real_time {
                        if vehicle.stop_id.as_ref().map(|s| s.as_str()) == Some(stop_id) {
                            let dest = vehicle.destination.clone().unwrap_or_else(|| "Unknown".to_string());
                            vehicles.push((line_ref.clone(), vehicle.clone(), dest));
                        }
                    }
                }
            }
        }
        
        // Sort by timestamp
        vehicles.sort_by_key(|(_, v, _)| v.timestamp.unwrap_or(i64::MAX));
        
        // Take next 10 vehicles (more since we're showing all lines)
        vehicles.into_iter().take(10).collect()
    }
    
    fn show_all_stops_browser(&mut self, ui: &mut Ui) {
        ui.heading("All Stops Browser");
        ui.separator();
        
        // Clone network data to avoid borrowing issues
        let network_opt = self.network.lock().unwrap().clone();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(network) = network_opt.as_ref() {
                ui.label(format!("Total stops: {}", network.stops.len()));
                ui.separator();
                
                let start = self.stops_page * self.stops_per_page;
                let end = (start + self.stops_per_page).min(network.stops.len());
                
                for stop in &network.stops[start..end] {
                    self.show_stop_card(ui, stop, &network);
                }
                
                ui.separator();
                ui.horizontal(|ui| {
                    if self.stops_page > 0 && ui.button("Previous").clicked() {
                        self.stops_page -= 1;
                    }
                    ui.label(format!("Page {} / {}", 
                        self.stops_page + 1, 
                        (network.stops.len() + self.stops_per_page - 1) / self.stops_per_page));
                    if end < network.stops.len() && ui.button("Next").clicked() {
                        self.stops_page += 1;
                    }
                });
            }
        });
    }
    
    fn show_all_lines_browser(&mut self, ui: &mut Ui) {
        ui.heading("All Lines Browser");
        ui.separator();
        
        // Clone network data to avoid borrowing issues
        let network_opt = self.network.lock().unwrap().clone();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(network) = network_opt.as_ref() {
                // Group lines by type
                let mut trams_brt: Vec<Line> = Vec::new();
                let mut buses: Vec<Line> = Vec::new();
                
                for line in &network.lines {
                    // Trams are A, B, C, D and BRT lines (Lianes)
                    if line.line_code.len() == 1 || line.line_name.contains("Liane") {
                        trams_brt.push(line.clone());
                    } else {
                        buses.push(line.clone());
                    }
                }
                
                ui.group(|ui| {
                    ui.strong(format!("🚊 Trams & BRT ({} lines)", trams_brt.len()));
                    ui.separator();
                    for line in &trams_brt {
                        self.show_line_card(ui, line);
                    }
                });
                
                ui.add_space(10.0);
                
                ui.group(|ui| {
                    ui.strong(format!("🚌 Buses ({} lines)", buses.len()));
                    ui.separator();
                    for line in &buses {
                        self.show_line_card(ui, line);
                    }
                });
            }
        });
    }
    
    fn show_cache_stats(&mut self, ui: &mut Ui) {
        ui.heading("Cache Statistics");
        ui.separator();
        
        // Clone cache to avoid borrowing issues
        let cache_opt = self.cache.lock().unwrap().clone();
        let mut should_refresh = false;
        
        if let Some(cache) = cache_opt.as_ref() {
            ui.group(|ui| {
                ui.label(RichText::new("Network Data").strong().size(16.0));
                ui.separator();
                ui.label(format!("📍 Stops: {}", cache.stops_metadata.len()));
                ui.label(format!("🚌 Lines: {}", cache.lines_metadata.len()));
                ui.label(format!("🎨 Line colors: {}", cache.line_colors.len()));
            });
            
            ui.add_space(10.0);
            
            ui.group(|ui| {
                ui.label(RichText::new("Real-Time Data").strong().size(16.0));
                ui.separator();
                ui.label(format!("🚍 Vehicle positions: {}", cache.real_time.len()));
                ui.label(format!("📅 Trip updates: {}", cache.trip_updates.len()));
                ui.label(format!("⚠️ Alerts: {}", cache.alerts.len()));
            });
            
            ui.add_space(10.0);
            
            ui.group(|ui| {
                ui.label(RichText::new("Cache Age").strong().size(16.0));
                ui.separator();
                
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                
                let static_age = now.saturating_sub(cache.last_static_update);
                let dynamic_age = now.saturating_sub(cache.last_dynamic_update);
                
                ui.label(format!("Static data age: {} seconds ({} min)", 
                    static_age, static_age / 60));
                ui.label(format!("Dynamic data age: {} seconds", dynamic_age));
                
                if cache.needs_static_refresh(3600) {
                    ui.colored_label(Color32::from_rgb(255, 165, 0), 
                        "⚠️ Static data needs refresh (>1 hour old)");
                }
                if cache.needs_dynamic_refresh(30) {
                    ui.colored_label(Color32::from_rgb(255, 165, 0), 
                        "⚠️ Dynamic data needs refresh (>30 seconds old)");
                }
            });
            
            ui.add_space(10.0);
            
            if ui.button("🔄 Refresh Data Now").clicked() {
                should_refresh = true;
            }
        } else {
            ui.label("No cache data available.");
        }
        
        if should_refresh {
            self.refresh_dynamic_data();
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn parse_hex_color(hex: &str) -> Color32 {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    // Default color if parsing fails
    Color32::from_rgb(100, 100, 100)
}

// ============================================================================
// Public entry point
// ============================================================================

pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "TBM Next Vehicle",
        options,
        Box::new(|cc| Ok(Box::new(NVTApp::new(cc)))),
    )
}
