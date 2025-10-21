// API models and data fetching for TBM (Transports Bordeaux Métropole) public transit service
// Official website: https://www.infotbm.com/
//
// API Endpoints:
// - Stop Discovery SIRI-Lite: https://bdx.mecatran.com/utw/ws/siri/2.0/bordeaux/stoppoints-discovery.json
// - Lines Discovery SIRI-Lite: https://bdx.mecatran.com/utw/ws/siri/2.0/bordeaux/lines-discovery.json
// - GTFS-RT Vehicles: https://bdx.mecatran.com/utw/ws/gtfsfeed/vehicles/bordeaux
// - GTFS-RT Alerts: https://bdx.mecatran.com/utw/ws/gtfsfeed/alerts/bordeaux
// - GTFS-RT Trip Updates: https://bdx.mecatran.com/utw/ws/gtfsfeed/realtime/bordeaux

use reqwest::blocking;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use gtfs_rt::FeedMessage;
use prost::Message;
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Europe::Paris;
use std::io::Read;
use std::io::Cursor;
use zip::ZipArchive;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use std::fs;

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertInfo {
    pub id: String,
    pub text: String,
    pub description: String,
    pub url: Option<String>,
    pub route_ids: Vec<String>,
    pub stop_ids: Vec<String>,
    pub active_period_start: Option<i64>,
    pub active_period_end: Option<i64>,
    pub severity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimeInfo {
    pub vehicle_id: String,
    pub trip_id: String,
    pub route_id: Option<String>,
    pub direction_id: Option<u32>,
    pub destination: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub stop_id: Option<String>,
    pub timestamp: Option<i64>,
    pub delay: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stop {
    pub stop_id: String,
    pub stop_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub lines: Vec<String>,
    pub alerts: Vec<AlertInfo>,
    pub real_time: Vec<RealTimeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub line_ref: String,
    pub line_name: String,
    pub line_code: String,
    pub destinations: Vec<(String, String)>,
    pub alerts: Vec<AlertInfo>,
    pub real_time: Vec<RealTimeInfo>,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct NetworkData {
    pub stops: Vec<Stop>,
    pub lines: Vec<Line>,
}

// ============================================================================
// GTFS Cache Structure (15-day persistence)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GTFSCache {
    pub routes: HashMap<String, String>,
    pub stops: Vec<(String, String, f64, f64)>,
    pub cached_at: u64,
}

impl GTFSCache {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let age_days = (now.saturating_sub(self.cached_at)) / 86400;
        age_days >= 15
    }

    pub fn cache_path() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("tbm_nvt");
        fs::create_dir_all(&path).ok();
        path.push("gtfs_cache.json");
        path
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::cache_path();
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| NVTError::FileError(format!("Failed to serialize cache: {}", e)))?;

        fs::write(&path, json)
            .map_err(|e| NVTError::FileError(format!("Failed to write cache: {}", e)))?;

        println!("✓ GTFS cache saved to: {:?}", path);
        Ok(())
    }

    pub fn load() -> Option<Self> {
        let path = Self::cache_path();

        if !path.exists() {
            println!("ℹ️  No GTFS cache found, will download fresh data");
            return None;
        }

        match fs::read_to_string(&path) {
            Ok(contents) => {
                match serde_json::from_str::<GTFSCache>(&contents) {
                    Ok(cache) => {
                        if cache.is_expired() {
                            println!("⚠️  GTFS cache expired (>15 days old), refreshing...");
                            None
                        } else {
                            let age_days = (SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs().saturating_sub(cache.cached_at)) / 86400;
                            println!("✓ GTFS cache loaded ({} days old)", age_days);
                            println!("  • {} routes with colors", cache.routes.len());
                            println!("  • {} stops cached", cache.stops.len());
                            Some(cache)
                        }
                    }
                    Err(e) => {
                        println!("⚠️  Failed to parse cache ({}), will refresh", e);
                        None
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to read cache file ({}), will refresh", e);
                None
            }
        }
    }
}

// ============================================================================
// Cache Structure for efficient refresh
// ============================================================================

#[derive(Debug, Clone)]
pub struct CachedNetworkData {
    pub stops_metadata: Vec<(String, String, f64, f64, Vec<String>)>,
    pub lines_metadata: Vec<(String, String, String, Vec<(String, String)>)>,
    pub line_colors: HashMap<String, String>,
    pub last_static_update: u64,
    pub alerts: Vec<AlertInfo>,
    pub real_time: Vec<RealTimeInfo>,
    pub trip_updates: Vec<gtfs_rt::TripUpdate>,
    pub last_dynamic_update: u64,
}

impl CachedNetworkData {
    pub fn new() -> Self {
        CachedNetworkData {
            stops_metadata: Vec::new(),
            lines_metadata: Vec::new(),
            line_colors: HashMap::new(),
            last_static_update: 0,
            alerts: Vec::new(),
            real_time: Vec::new(),
            trip_updates: Vec::new(),
            last_dynamic_update: 0,
        }
    }

    pub fn needs_static_refresh(&self, max_age_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.last_static_update) > max_age_seconds
    }

    pub fn needs_dynamic_refresh(&self, max_age_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.last_dynamic_update) > max_age_seconds
    }

    pub fn to_network_data(&self) -> NetworkData {
        NVTModels::build_network_data(
            self.stops_metadata.clone(),
            self.lines_metadata.clone(),
            self.alerts.clone(),
            self.real_time.clone(),
            self.trip_updates.clone(),
            self.line_colors.clone(),
        )
    }
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug)]
pub enum NVTError {
    NetworkError(String),
    ParseError(String),
    FileError(String),
}

impl std::fmt::Display for NVTError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NVTError::NetworkError(e) => write!(f, "Network error: {}", e),
            NVTError::ParseError(e) => write!(f, "Parse error: {}", e),
            NVTError::FileError(e) => write!(f, "File error: {}", e),
        }
    }
}

impl std::error::Error for NVTError {}

pub type Result<T> = std::result::Result<T, NVTError>;

// ============================================================================
// Main Implementation
// ============================================================================

pub struct NVTModels;

impl NVTModels {
    const API_KEY: &'static str = "opendata-bordeaux-metropole-flux-gtfs-rt";
    const BASE_URL: &'static str = "https://bdx.mecatran.com/utw/ws";
    const STATIC_DATA_MAX_AGE: u64 = 3600;
    const DYNAMIC_DATA_MAX_AGE: u64 = 30;
    const REQUEST_TIMEOUT_SECS: u64 = 15;

    pub fn initialize_cache() -> Result<CachedNetworkData> {
        println!("🔄 Initializing network data cache...");
        println!("   This may take a moment...");

        let stops = Self::fetch_stops().map_err(|e| {
            NVTError::NetworkError(format!("Failed to fetch stops: {}", e))
        })?;
        println!("   ✓ Loaded {} stops", stops.len());

        let lines = Self::fetch_lines().map_err(|e| {
            NVTError::NetworkError(format!("Failed to fetch lines: {}", e))
        })?;
        println!("   ✓ Loaded {} lines", lines.len());

        let line_colors = Self::load_line_colors().map_err(|e| {
            println!("   ⚠️  Warning: Could not load line colors ({})", e);
            println!("   Continuing with default colors...");
            e
        }).unwrap_or_default();
        println!("   ✓ Loaded {} line colors", line_colors.len());

        let alerts = Self::fetch_alerts().unwrap_or_else(|e| {
            println!("   ⚠️  Warning: Could not fetch alerts ({})", e);
            Vec::new()
        });
        println!("   ✓ Loaded {} alerts", alerts.len());

        let real_time = Self::fetch_vehicle_positions().unwrap_or_else(|e| {
            println!("   ⚠️  Warning: Could not fetch vehicle positions ({})", e);
            Vec::new()
        });
        println!("   ✓ Loaded {} vehicle positions", real_time.len());

        let trip_updates = Self::fetch_trip_updates().unwrap_or_else(|e| {
            println!("   ⚠️  Warning: Could not fetch trip updates ({})", e);
            Vec::new()
        });
        println!("   ✓ Loaded {} trip updates", trip_updates.len());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        println!("\n✓ Cache initialized successfully!");
        println!("  • {} stops, {} lines", stops.len(), lines.len());
        println!("  • {} vehicles tracked, {} alerts", real_time.len(), alerts.len());

        Ok(CachedNetworkData {
            stops_metadata: stops,
            lines_metadata: lines,
            line_colors,
            last_static_update: now,
            alerts,
            real_time,
            trip_updates,
            last_dynamic_update: now,
        })
    }

    pub fn refresh_dynamic_data(cache: &mut CachedNetworkData) -> Result<()> {
        cache.alerts = Self::fetch_alerts().unwrap_or_else(|e| {
            eprintln!("⚠️  Warning: Could not fetch alerts ({})", e);
            cache.alerts.clone()
        });

        cache.real_time = Self::fetch_vehicle_positions().unwrap_or_else(|e| {
            eprintln!("⚠️  Warning: Could not fetch vehicle positions ({})", e);
            cache.real_time.clone()
        });

        cache.trip_updates = Self::fetch_trip_updates().unwrap_or_else(|e| {
            eprintln!("⚠️  Warning: Could not fetch trip updates ({})", e);
            cache.trip_updates.clone()
        });

        cache.last_dynamic_update = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(())
    }

    pub fn refresh_static_data(cache: &mut CachedNetworkData) -> Result<()> {
        println!("🔄 Refreshing static network data...");

        cache.stops_metadata = Self::fetch_stops()?;
        cache.lines_metadata = Self::fetch_lines()?;
        cache.line_colors = Self::load_line_colors().unwrap_or_default();

        cache.last_static_update = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        println!("✓ Static data refreshed!");

        Ok(())
    }

    pub fn smart_refresh(cache: &mut CachedNetworkData) -> Result<()> {
        Self::refresh_dynamic_data(cache)?;

        if cache.needs_static_refresh(Self::STATIC_DATA_MAX_AGE) {
            Self::refresh_static_data(cache)?;
        }

        Ok(())
    }

    fn fetch_stops() -> Result<Vec<(String, String, f64, f64, Vec<String>)>> {
        let url = format!(
            "{}/siri/2.0/bordeaux/stoppoints-discovery.json?AccountKey={}",
            Self::BASE_URL,
            Self::API_KEY
        );

        let client = blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| NVTError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client.get(&url)
            .send()
            .map_err(|e| NVTError::NetworkError(format!("Failed to fetch stops: {}. Check your internet connection.", e)))?;

        if !response.status().is_success() {
            return Err(NVTError::NetworkError(format!("API returned error: {}", response.status())));
        }

        let body = response.text()
            .map_err(|e| NVTError::NetworkError(format!("Failed to read response: {}", e)))?;

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| NVTError::ParseError(format!("Invalid JSON response: {}", e)))?;

        let stop_points = json["Siri"]["StopPointsDelivery"]["AnnotatedStopPointRef"]
            .as_array()
            .ok_or_else(|| NVTError::ParseError("Missing or invalid stop points data in API response".to_string()))?;

        let stops: Vec<_> = stop_points
            .iter()
            .filter_map(|stop| {
                let full_id = stop["StopPointRef"]["value"].as_str()?;
                let stop_id = Self::extract_stop_id(full_id)?;
                let stop_name = stop["StopName"]["value"].as_str()?.to_string();
                let latitude = stop["Location"]["latitude"].as_f64()?;
                let longitude = stop["Location"]["longitude"].as_f64()?;
                let lines = stop["Lines"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|line| line["value"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                Some((stop_id, stop_name, latitude, longitude, lines))
            })
            .collect();

        if stops.is_empty() {
            return Err(NVTError::ParseError("No valid stops found in API response".to_string()));
        }

        Ok(stops)
    }

    fn fetch_lines() -> Result<Vec<(String, String, String, Vec<(String, String)>)>> {
        let url = format!(
            "{}/siri/2.0/bordeaux/lines-discovery.json?AccountKey={}",
            Self::BASE_URL,
            Self::API_KEY
        );

        let client = blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| NVTError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client.get(&url)
            .send()
            .map_err(|e| NVTError::NetworkError(format!("Failed to fetch lines: {}. Check your internet connection.", e)))?;

        if !response.status().is_success() {
            return Err(NVTError::NetworkError(format!("API returned error: {}", response.status())));
        }

        let body = response.text()
            .map_err(|e| NVTError::NetworkError(format!("Failed to read response: {}", e)))?;

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| NVTError::ParseError(format!("Invalid JSON response: {}", e)))?;

        let line_refs = json["Siri"]["LinesDelivery"]["AnnotatedLineRef"]
            .as_array()
            .ok_or_else(|| NVTError::ParseError("Missing or invalid lines data in API response".to_string()))?;

        let lines: Vec<_> = line_refs
            .iter()
            .filter_map(|line| {
                let line_ref = line["LineRef"]["value"].as_str()?.to_string();
                let line_name = line["LineName"][0]["value"].as_str()?.to_string();
                let line_code = line["LineCode"]["value"].as_str()?.to_string();
                let destinations = line["Destinations"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|dest| {
                                let direction = dest["DirectionRef"]["value"].as_str()?.to_string();
                                let place = dest["PlaceName"][0]["value"].as_str()?.to_string();
                                Some((direction, place))
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                Some((line_ref, line_name, line_code, destinations))
            })
            .collect();

        if lines.is_empty() {
            return Err(NVTError::ParseError("No valid lines found in API response".to_string()));
        }

        Ok(lines)
    }

    fn fetch_alerts() -> Result<Vec<AlertInfo>> {
        let url = format!(
            "{}/gtfsfeed/alerts/bordeaux?apiKey={}",
            Self::BASE_URL,
            Self::API_KEY
        );

        let client = blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| NVTError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client.get(&url)
            .send()
            .map_err(|e| NVTError::NetworkError(format!("Failed to fetch alerts: {}", e)))?;

        let body = response.bytes()
            .map_err(|e| NVTError::NetworkError(format!("Failed to read alerts response: {}", e)))?;

        let feed = FeedMessage::decode(&*body)
            .map_err(|e| NVTError::ParseError(format!("Failed to decode alerts feed: {}", e)))?;

        let alerts = feed
            .entity
            .into_iter()
            .filter_map(|entity| {
                entity.alert.map(|alert| {
                    let header_text = alert
                        .header_text
                        .and_then(|h| h.translation.first().map(|t| t.text.clone()))
                        .unwrap_or_else(|| "No title".to_string());

                    let description_text = alert
                        .description_text
                        .and_then(|d| d.translation.first().map(|t| t.text.clone()))
                        .unwrap_or_else(|| "No description available".to_string());

                    let url = alert
                        .url
                        .and_then(|u| u.translation.first().map(|t| t.text.clone()));

                    let mut route_ids = Vec::new();
                    let mut stop_ids = Vec::new();

                    for informed_entity in alert.informed_entity {
                        if let Some(route_id) = informed_entity.route_id {
                            route_ids.push(route_id);
                        }
                        if let Some(stop_id) = informed_entity.stop_id {
                            // Use raw stop_id directly for alerts
                            stop_ids.push(stop_id);
                        }
                    }

                    let (start, end) = alert.active_period
                        .first()
                        .map(|period| {
                            (
                                period.start.map(|s| s as i64),
                                period.end.map(|e| e as i64)
                            )
                        })
                        .unwrap_or((None, None));

                    let severity = alert.severity_level.unwrap_or(0) as u32;

                    AlertInfo {
                        id: entity.id,
                        text: header_text,
                        description: description_text,
                        url,
                        route_ids,
                        stop_ids,
                        active_period_start: start,
                        active_period_end: end,
                        severity,
                    }
                })
            })
            .collect();

        Ok(alerts)
    }

    fn fetch_vehicle_positions() -> Result<Vec<RealTimeInfo>> {
        let url = format!(
            "{}/gtfsfeed/vehicles/bordeaux?apiKey={}",
            Self::BASE_URL,
            Self::API_KEY
        );

        let client = blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| NVTError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client.get(&url)
            .send()
            .map_err(|e| NVTError::NetworkError(format!("Failed to fetch vehicle positions: {}", e)))?;

        let body = response.bytes()
            .map_err(|e| NVTError::NetworkError(format!("Failed to read vehicles response: {}", e)))?;

        let feed = FeedMessage::decode(&*body)
            .map_err(|e| NVTError::ParseError(format!("Failed to decode vehicles feed: {}", e)))?;

        let real_time: Vec<RealTimeInfo> = feed
            .entity
            .into_iter()
            .filter_map(|entity| {
                entity.vehicle.map(|vehicle| {
                    let vehicle_id = vehicle
                        .vehicle
                        .as_ref()
                        .and_then(|v| v.id.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let trip_id = vehicle
                        .trip
                        .as_ref()
                        .and_then(|t| t.trip_id.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let route_id = vehicle
                        .trip
                        .as_ref()
                        .and_then(|t| t.route_id.clone());

                    let direction_id = vehicle
                        .trip
                        .as_ref()
                        .and_then(|t| t.direction_id);

                    let destination = vehicle
                        .vehicle
                        .as_ref()
                        .and_then(|v| v.label.clone());

                    let (latitude, longitude) = vehicle
                        .position
                        .as_ref()
                        .map(|p| (p.latitude as f64, p.longitude as f64))
                        .unwrap_or((0.0, 0.0));

                    // Use raw stop_id - no extraction needed for vehicles
                    let stop_id = vehicle.stop_id.clone();
                    let timestamp = vehicle.timestamp.map(|ts| ts as i64);

                    RealTimeInfo {
                        vehicle_id,
                        trip_id,
                        route_id,
                        direction_id,
                        destination,
                        latitude,
                        longitude,
                        stop_id,
                        timestamp,
                        delay: None,
                    }
                })
            })
            .collect();

        Ok(real_time)
    }

    fn fetch_trip_updates() -> Result<Vec<gtfs_rt::TripUpdate>> {
        let url = format!(
            "{}/gtfsfeed/realtime/bordeaux?apiKey={}",
            Self::BASE_URL,
            Self::API_KEY
        );

        let client = blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| NVTError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client.get(&url)
            .send()
            .map_err(|e| NVTError::NetworkError(format!("Failed to fetch trip updates: {}", e)))?;

        let body = response.bytes()
            .map_err(|e| NVTError::NetworkError(format!("Failed to read trip updates response: {}", e)))?;

        let feed = FeedMessage::decode(&*body)
            .map_err(|e| NVTError::ParseError(format!("Failed to decode trip updates feed: {}", e)))?;

        let updates = feed
            .entity
            .into_iter()
            .filter_map(|entity| entity.trip_update)
            .collect();

        Ok(updates)
    }

    fn download_and_read_routes() -> Result<HashMap<String, String>> {
        if let Some(cache) = GTFSCache::load() {
            return Ok(cache.routes);
        }

        println!("📥 Downloading fresh GTFS data (this may take a moment)...");
        let gtfs_url = "https://transport.data.gouv.fr/resources/83024/download";

        let client = blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| NVTError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client.get(gtfs_url)
            .send()
            .map_err(|e| NVTError::NetworkError(format!("Failed to download GTFS: {}. Check your internet connection.", e)))?;

        if !response.status().is_success() {
            return Err(NVTError::NetworkError(format!("GTFS download failed with status: {}", response.status())));
        }

        let zip_bytes = response.bytes()
            .map_err(|e| NVTError::NetworkError(format!("Failed to read GTFS zip: {}", e)))?;

        println!("✓ Downloaded {} KB, extracting...", zip_bytes.len() / 1024);

        let cursor = Cursor::new(zip_bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| NVTError::ParseError(format!("Failed to open GTFS zip archive: {}", e)))?;

        let mut routes_file = archive.by_name("routes.txt")
            .map_err(|e| NVTError::FileError(format!("routes.txt not found in GTFS archive: {}", e)))?;

        let mut routes_contents = String::new();
        routes_file.read_to_string(&mut routes_contents)
            .map_err(|e| NVTError::FileError(format!("Failed to read routes.txt: {}", e)))?;

        drop(routes_file);

        let stops_contents = match archive.by_name("stops.txt") {
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents).ok();
                Some(contents)
            }
            Err(_) => None,
        };

        let mut color_map = HashMap::new();
        let mut rdr = csv::Reader::from_reader(routes_contents.as_bytes());

        for result in rdr.records() {
            match result {
                Ok(record) => {
                    if let (Some(route_id), Some(route_color)) = (record.get(0), record.get(5)) {
                        if !route_color.is_empty() && route_color.len() == 6 {
                            color_map.insert(route_id.to_string(), route_color.to_string());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Warning: Skipping invalid route record: {}", e);
                }
            }
        }

        let mut stops_data = Vec::new();
        if let Some(contents) = stops_contents {
            let mut stops_rdr = csv::Reader::from_reader(contents.as_bytes());

            for result in stops_rdr.records() {
                if let Ok(record) = result {
                    if let (Some(stop_id), Some(stop_name), Some(lat_str), Some(lon_str)) =
                        (record.get(0), record.get(2), record.get(4), record.get(5)) {
                        if let (Ok(lat), Ok(lon)) = (lat_str.parse::<f64>(), lon_str.parse::<f64>()) {
                            stops_data.push((
                                stop_id.to_string(),
                                stop_name.to_string(),
                                lat,
                                lon,
                            ));
                        }
                    }
                }
            }
        }

        let cache = GTFSCache {
            routes: color_map.clone(),
            stops: stops_data,
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        if let Err(e) = cache.save() {
            eprintln!("⚠️  Warning: Could not save GTFS cache: {}", e);
        }

        println!("✓ Loaded {} route colors", color_map.len());
        println!("✓ Cached {} stops for future use", cache.stops.len());

        Ok(color_map)
    }

    fn load_line_colors() -> Result<HashMap<String, String>> {
        Self::download_and_read_routes()
    }

    /// Build complete network data with all associations - OPTIMIZED
    pub fn build_network_data(
        stops_data: Vec<(String, String, f64, f64, Vec<String>)>,
        lines_data: Vec<(String, String, String, Vec<(String, String)>)>,
        alerts: Vec<AlertInfo>,
        real_time: Vec<RealTimeInfo>,
        trip_updates: Vec<gtfs_rt::TripUpdate>,
        line_color_map: HashMap<String, String>,
    ) -> NetworkData {
        let line_destinations_map: HashMap<String, Vec<(String, String)>> = lines_data
            .iter()
            .filter_map(|(ref_, _, _, destinations)| {
                let line_id = Self::extract_line_id(ref_)?;
                Some((line_id.to_string(), destinations.clone()))
            })
            .collect();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Allow arrivals up to 2 minutes in the past (grace period for vehicles at stop)
        let grace_period = 120; // seconds
        let cutoff_time = now - grace_period;

        let mut trip_updates_by_stop: HashMap<String, Vec<(String, Option<String>, Option<u32>, Option<i32>, Option<i64>)>> = HashMap::new();

        for trip_update in &trip_updates {
            let trip_id = trip_update.trip.trip_id.clone().unwrap_or_else(|| "Unknown".to_string());
            let route_id = trip_update.trip.route_id.clone();
            let direction_id = trip_update.trip.direction_id;

            for stu in &trip_update.stop_time_update {
                if let Some(stop_id_raw) = &stu.stop_id {
                    let delay = stu.arrival.as_ref().and_then(|a| a.delay)
                        .or_else(|| stu.departure.as_ref().and_then(|d| d.delay));
                    let time = stu.arrival.as_ref().and_then(|a| a.time)
                        .or_else(|| stu.departure.as_ref().and_then(|d| d.time))
                        .map(|t| t as i64);

                    if let Some(arrival_time) = time {
                        // Include arrivals within grace period OR in the future
                        if arrival_time >= cutoff_time {
                            let data = (
                                trip_id.clone(),
                                route_id.clone(),
                                direction_id,
                                delay,
                                time,
                            );

                            // Index by raw stop_id (e.g., "5220")
                            trip_updates_by_stop
                                .entry(stop_id_raw.clone())
                                .or_insert_with(Vec::new)
                                .push(data.clone());

                            // ALSO index by extracted stop_id (in case SIRI uses different format)
                            if let Some(extracted) = Self::extract_stop_id(stop_id_raw) {
                                if extracted != *stop_id_raw {
                                    trip_updates_by_stop
                                        .entry(extracted)
                                        .or_insert_with(Vec::new)
                                        .push(data);
                                }
                            }
                        }
                    }
                }
            }
        }

        let stops: Vec<Stop> = stops_data
            .into_iter()
            .map(|(id, name, lat, lon, line_refs)| {
                let mut stop_rt: Vec<RealTimeInfo> = real_time
                    .iter()
                    .filter(|rt| {
                        rt.stop_id
                            .as_ref()
                            .map(|sid| sid == &id)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                // Add trip updates (scheduled arrivals)
                if let Some(scheduled_arrivals) = trip_updates_by_stop.get(&id) {
                    for (trip_id, route_id, direction_id, delay, time) in scheduled_arrivals {
                        let destination = route_id.as_ref().and_then(|rid| {
                            line_destinations_map.get(rid).and_then(|destinations| {
                                direction_id.and_then(|dir_id| {
                                    destinations.iter()
                                        .find(|(dir_ref, _)| dir_ref == &dir_id.to_string())
                                        .map(|(_, place)| place.clone())
                                })
                            })
                        });

                        stop_rt.push(RealTimeInfo {
                            vehicle_id: "scheduled".to_string(),
                            trip_id: trip_id.clone(),
                            route_id: route_id.clone(),
                            direction_id: *direction_id,
                            destination,
                            latitude: lat,
                            longitude: lon,
                            stop_id: Some(id.clone()),
                            timestamp: *time,
                            delay: *delay,
                        });
                    }
                }

                // Keep arrivals within grace period OR future arrivals
                stop_rt.retain(|rt| {
                    if let Some(ts) = rt.timestamp {
                        ts >= cutoff_time
                    } else {
                        true
                    }
                });

                // Sort by timestamp
                stop_rt.sort_by_key(|rt| rt.timestamp.unwrap_or(i64::MAX));

                // OPTIONAL: Limit to next N arrivals to avoid overwhelming UI
                const MAX_ARRIVALS_PER_STOP: usize = 10;
                if stop_rt.len() > MAX_ARRIVALS_PER_STOP {
                    stop_rt.truncate(MAX_ARRIVALS_PER_STOP);
                }

                let stop_alerts: Vec<AlertInfo> = alerts
                    .iter()
                    .filter(|alert| alert.stop_ids.contains(&id))
                    .cloned()
                    .collect();

                Stop {
                    stop_id: id,
                    stop_name: name,
                    latitude: lat,
                    longitude: lon,
                    lines: line_refs,
                    alerts: stop_alerts,
                    real_time: stop_rt,
                }
            })
            .collect();

        let lines: Vec<Line> = lines_data
            .into_iter()
            .map(|(ref_, name, code, destinations)| {
                let line_id = Self::extract_line_id(&ref_).unwrap_or("");
                let color = line_color_map
                    .get(line_id)
                    .cloned()
                    .unwrap_or_else(|| "808080".to_string());

                let line_alerts: Vec<AlertInfo> = alerts
                    .iter()
                    .filter(|alert| {
                        alert.route_ids.contains(&code) ||
                            alert.route_ids.contains(&line_id.to_string())
                    })
                    .cloned()
                    .collect();

                let mut line_rt: Vec<RealTimeInfo> = real_time
                    .iter()
                    .filter(|rt| {
                        rt.route_id
                            .as_ref()
                            .map(|route| route == line_id)
                            .unwrap_or(false)
                    })
                    .filter(|rt| {
                        if let Some(ts) = rt.timestamp {
                            ts >= cutoff_time
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect();

                line_rt.sort_by_key(|rt| rt.timestamp.unwrap_or(i64::MAX));

                Line {
                    line_ref: ref_,
                    line_name: name,
                    line_code: code,
                    destinations,
                    alerts: line_alerts,
                    real_time: line_rt,
                    color,
                }
            })
            .collect();

        NetworkData { stops, lines }
    }

    fn extract_stop_id(full_id: &str) -> Option<String> {
        if full_id.contains("BP:") {
            full_id
                .split("BP:")
                .nth(1)?
                .split(':')
                .next()
                .map(String::from)
        } else if full_id.contains(':') {
            let parts: Vec<&str> = full_id.split(':').collect();
            if parts.len() >= 2 {
                Some(parts[parts.len() - 2].to_string())
            } else {
                Some(full_id.to_string())
            }
        } else {
            Some(full_id.to_string())
        }
    }

    pub fn extract_line_id(line_ref: &str) -> Option<&str> {
        line_ref.split(':').nth(2)
    }

    pub fn get_line_color(line_code: &str, network: &NetworkData) -> String {
        network
            .lines
            .iter()
            .find(|l| l.line_code.eq_ignore_ascii_case(line_code))
            .map(|l| l.color.clone())
            .unwrap_or_else(|| "808080".to_string())
    }

    pub fn parse_hex_color(hex_color: &str) -> (u8, u8, u8) {
        if hex_color.len() != 6 {
            return (128, 128, 128);
        }
        let r = u8::from_str_radix(&hex_color[0..2], 16).unwrap_or(128);
        let g = u8::from_str_radix(&hex_color[2..4], 16).unwrap_or(128);
        let b = u8::from_str_radix(&hex_color[4..6], 16).unwrap_or(128);
        (r, g, b)
    }

    pub fn get_line_color_rgb(line_code: &str, network: &NetworkData) -> (u8, u8, u8) {
        let hex_color = Self::get_line_color(line_code, network);
        Self::parse_hex_color(&hex_color)
    }

    pub fn get_stop_by_name<'a>(name: &str, network: &'a NetworkData) -> Option<&'a Stop> {
        network.stops.iter().find(|s| s.stop_name.eq_ignore_ascii_case(name))
    }

    pub fn get_line_by_name<'a>(name: &str, network: &'a NetworkData) -> Option<&'a Line> {
        network.lines.iter().find(|l| l.line_name.eq_ignore_ascii_case(name))
    }

    pub fn get_line_by_route_id<'a>(route_id: &str, network: &'a NetworkData) -> Option<&'a Line> {
        network
            .lines
            .iter()
            .find(|l| Self::extract_line_id(&l.line_ref) == Some(route_id))
    }

    pub fn get_stops_for_line<'a>(line_ref: &str, network: &'a NetworkData) -> Vec<&'a Stop> {
        network
            .stops
            .iter()
            .filter(|s| s.lines.iter().any(|l| l == line_ref))
            .collect()
    }

    pub fn get_next_vehicles_for_stop<'a>(
        stop_id: &str,
        network: &'a NetworkData,
    ) -> Vec<&'a RealTimeInfo> {
        network
            .stops
            .iter()
            .find(|s| s.stop_id == stop_id)
            .map(|stop| {
                let mut vehicles: Vec<&RealTimeInfo> = stop.real_time.iter().collect();
                vehicles.sort_by_key(|rt| rt.timestamp.unwrap_or(i64::MAX));
                vehicles
            })
            .unwrap_or_default()
    }

    pub fn format_timestamp(timestamp: i64) -> String {
        match Utc.timestamp_opt(timestamp, 0).single() {
            Some(dt) => {
                let paris_time = dt.with_timezone(&Paris);
                paris_time.format("%H:%M:%S").to_string()
            }
            None => "??:??:??".to_string(),
        }
    }

    pub fn format_timestamp_full(timestamp: i64) -> String {
        match Utc.timestamp_opt(timestamp, 0).single() {
            Some(dt) => {
                let paris_time = dt.with_timezone(&Paris);
                paris_time.format("%Y-%m-%d %H:%M:%S").to_string()
            }
            None => format!("Invalid timestamp: {}", timestamp),
        }
    }

    pub fn get_current_timestamp() -> i64 {
        Utc::now().timestamp()
    }

    pub fn get_cache_stats(cache: &CachedNetworkData) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let static_age = now.saturating_sub(cache.last_static_update);
        let dynamic_age = now.saturating_sub(cache.last_dynamic_update);

        format!(
            "📊 Cache Statistics:\n\
             • Stops: {} | Lines: {} | Colors: {}\n\
             • Vehicles tracked: {} | Alerts (Active or Future): {}\n\
             • Static data age: {}s | Dynamic data age: {}s\n\
             • Last update: {}",
            cache.stops_metadata.len(),
            cache.lines_metadata.len(),
            cache.line_colors.len(),
            cache.real_time.len(),
            cache.alerts.len(),
            static_age,
            dynamic_age,
            Self::format_timestamp_full(cache.last_dynamic_update as i64)
        )
    }
}