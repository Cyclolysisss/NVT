# 🚊 NVT - **N**ext **V**ehicle **T**BM

[![Rust](https://img.shields.io/badge/Rust-100%25-orange?style=flat&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey)](https://github.com/Cyclolysisss/NVT)

A powerful, real-time transit tracking application for Bordeaux Métropole's public transportation network (TBM). Monitor buses, trams, and BRT lines with live GPS tracking, service alerts, and comprehensive network information.

## ✨ Features

### 🚀 Real-Time Tracking
- **Live Vehicle Positions**: GPS-based tracking of all TBM vehicles in real-time
- **Accurate Arrival Times**: Precise ETAs using GTFS-RT data
- **Auto-Refresh Mode**: Continuous 30-second updates for monitoring
- **Delay Indicators**: Visual status indicators for on-time, delayed, or early arrivals

### 📊 Comprehensive Network Data
- **700+ Stops**: Complete coverage of all TBM network stops
- **100+ Lines**: All tram, bus, and BRT lines with route information
- **Service Alerts**: Real-time notifications about disruptions and changes
- **Line Colors**: Authentic TBM branding with official line colors

### ⚡ Smart Caching System
- **15-Day GTFS Cache**: Reduces bandwidth and speeds up loading
- **Intelligent Refresh**: Automatic updates only when needed
- **Offline Fallback**: Uses cached data when network is unavailable
- **Optimized Performance**: Separate static and dynamic data management

### 🎨 User-Friendly Interface
- **Colorized Output**: Line codes displayed in official TBM colors
- **Intuitive Navigation**: Simple numbered menu system
- **Smart Search**: Partial name matching for stops and lines
- **Rich Information Display**: Detailed vehicle, stop, and line information

## 📋 Table of Contents

- [Installation](#-installation)
- [Usage](#-usage)
- [Data Sources](#-data-sources)
- [Architecture](#-architecture)
- [Configuration](#-configuration)
- [Contributing](#-contributing)
- [Acknowledgments](#-acknowledgments)

## 🔧 Installation

### Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Internet Connection**: Required for initial data download and real-time updates

### Build from Source

```bash
# Clone the repository
git clone https://github.com/Cyclolysisss/NVT.git
cd NVT

# Build the project
cargo build --release

# Run the application
cargo run --release
```

### Binary Installation

Download the latest pre-built binary from the [Releases](https://github.com/Cyclolysisss/NVT/releases) page.

```bash
# Linux/macOS
chmod +x nvt
./nvt

# Windows
nvt.exe
```

## 🚀 Usage

### Quick Start

1. **Launch the application**
   ```bash
   cargo run --release
   ```

2. **Select a line** (Option 1)
    - Enter line code (e.g., `A`, `C`, `1`, `23`)
    - Or enter full name (e.g., `Tram A`)

3. **Select a stop** (Option 2)
    - Enter partial or full stop name
    - Choose from multiple matches if needed

4. **View real-time arrivals** (Option 3)
    - See next vehicles with live ETAs
    - Auto-refreshes every 30 seconds
    - Press Enter to exit refresh mode

### Menu Options

```
📋 MENU OPTIONS
  1️⃣  Select a line
  2️⃣  Select a stop
  3️⃣  Show next vehicles in real-time 🔄
  4️⃣  Browse all stops
  5️⃣  Browse all lines
  6️⃣  Show cache statistics 📊
  0️⃣  Quit application
```

### Example Workflow

```
➜ Select Option: 1
🚌 Enter line name or code: A

✓ Line selected: A - Tram A
  🎯 Destinations:
     → Outbound : La Gardette Bassens Carbon Blanc
     ← Inbound : Le Haillan Rostand

➜ Select Option: 2
📍 Enter stop name: hotel de ville

✓ Stop selected: Hôtel de Ville
  🚌 Lines serving this stop: A B

➜ Select Option: 3
🔄 AUTO-REFRESH MODE - Update #1
📅 Monday, October 21, 2025 at 15:34:28 CEST

  1. A Tram A
     🎯 Direction: La Gardette Bassens Carbon Blanc
     ⏰ Time: 15:37:30 (🟢 3 min)
     ⏱️  Status: 🟢 On time
     📊 Source: Real-time GPS tracking
     🚌 Vehicle ID: 1234
```

### Advanced Features

#### Filtering by Line
Select a line first, then view only vehicles on that line at any stop.

#### Service Alerts
Automatically displays active and future alerts for selected stops and lines.

#### Cache Management
View cache statistics to monitor data freshness and performance.

## 📡 Data Sources

### Official TBM Open Data APIs

The application uses official TBM (Transports Bordeaux Métropole) data sources:

#### SIRI-Lite APIs
- **Stops Discovery**: `https://bdx.mecatran.com/utw/ws/siri/2.0/bordeaux/stoppoints-discovery.json`
- **Lines Discovery**: `https://bdx.mecatran.com/utw/ws/siri/2.0/bordeaux/lines-discovery.json`

#### GTFS-RT Feeds
- **Vehicle Positions**: Real-time GPS locations
- **Trip Updates**: Arrival/departure predictions
- **Service Alerts**: Network disruptions and changes

#### GTFS Static Data
- **Routes**: Line colors and route information
- **Stops**: Comprehensive stop database

### Data Update Frequency

| Data Type | Update Interval | Cache Duration |
|-----------|----------------|----------------|
| Vehicle Positions | 30 seconds | N/A (real-time) |
| Trip Updates | 30 seconds | N/A (real-time) |
| Service Alerts | 30 seconds | N/A (real-time) |
| Stops/Lines Metadata | 1 hour | 1 hour |
| GTFS Static Data | On-demand | 15 days |

## 🏗️ Architecture

### Project Structure

```
NVT/
├── src/
│   ├── main.rs              # Application entry point & error handling
│   ├── nvt_models.rs        # Data models & API fetching
│   ├── nvt_views.rs         # User interface & display logic
│   └── nvt_controllers.rs   # Business logic & app flow
├── Cargo.toml               # Dependencies & metadata
└── README.md
```

### Module Overview

#### `nvt_models.rs` - Data Layer
- **API Integration**: Fetches data from TBM endpoints
- **Data Structures**: `Stop`, `Line`, `RealTimeInfo`, `AlertInfo`
- **Caching System**: Intelligent cache management
- **GTFS Processing**: Handles GTFS-RT protobuf decoding

#### `nvt_views.rs` - Presentation Layer
- **UI Components**: Menus, prompts, and formatted output
- **Color Rendering**: ANSI color codes for line branding
- **Information Display**: Vehicle, stop, and line information
- **Error Messages**: User-friendly error handling

#### `nvt_controllers.rs` - Business Logic
- **Application Flow**: Main menu loop and navigation
- **Selection Handling**: Line and stop selection logic
- **Auto-Refresh**: Real-time update mechanism
- **Input Processing**: User input validation and parsing

### Design Patterns

- **MVC Architecture**: Separation of concerns (Models, Views, Controllers)
- **Caching Strategy**: Two-tier cache (static + dynamic)
- **Error Handling**: Custom `Result` type with `NVTError` enum
- **Lazy Loading**: Data fetched only when needed

## ⚙️ Configuration

### Cache Location

The GTFS cache is stored in the system cache directory:

- **Linux**: `~/.cache/tbm_nvt/gtfs_cache.json`
- **macOS**: `~/Library/Caches/tbm_nvt/gtfs_cache.json`
- **Windows**: `%LOCALAPPDATA%\tbm_nvt\gtfs_cache.json`

### API Configuration

API endpoints and keys are configured in `nvt_models.rs`:

```rust
const API_KEY: &'static str = "opendata-bordeaux-metropole-flux-gtfs-rt";
const BASE_URL: &'static str = "https://bdx.mecatran.com/utw/ws";
```

### Timeouts

```rust
const REQUEST_TIMEOUT_SECS: u64 = 15;  // API request timeout
const STATIC_DATA_MAX_AGE: u64 = 3600; // 1 hour
const DYNAMIC_DATA_MAX_AGE: u64 = 30;  // 30 seconds
```

## 🔧 Dependencies

### Core Libraries

```toml
[dependencies]
reqwest = { version = "0.11", features = ["blocking"] }  # HTTP client
serde = { version = "1.0", features = ["derive"] }       # Serialization
serde_json = "1.0"                                        # JSON parsing
gtfs-rt = "0.4"                                          # GTFS-RT decoder
prost = "0.12"                                           # Protobuf support
chrono = "0.4"                                           # Date/time handling
chrono-tz = "0.8"                                        # Timezone support
csv = "1.3"                                              # CSV parsing
zip = "0.6"                                              # GTFS archive extraction
dirs = "5.0"                                             # System directories
```

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

### Reporting Issues

1. Check existing issues first
2. Provide detailed error messages
3. Include steps to reproduce
4. Mention your OS and Rust version

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style

- Follow Rust standard conventions
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes
- Add documentation for public functions

## 🙏 Acknowledgments

- **TBM (Transports Bordeaux Métropole)**: For providing open data APIs
- **Bordeaux Métropole**: For supporting open data initiatives
- **Mecatran**: For hosting and maintaining the TBM data feeds
- **Rust Community**: For excellent libraries and documentation

## 📞 Contact & Support

- **Issues**: [GitHub Issues](https://github.com/Cyclolysisss/NVT/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Cyclolysisss/NVT/discussions)
- **TBM Official Site**: [infotbm.com](https://www.infotbm.com/)
- **Open Data Portal**: [transport.data.gouv.fr](https://transport.data.gouv.fr/)

## 🗺️ Roadmap

### Planned Features

- [ ] Offline mode with cached data and schedulded trips
- [ ] Trip planning functionality
- [ ] Favorite stops/lines
- [ ] Notifications for specific arrivals
- [ ] Export data to CSV/JSON
- [ ] Historical data analysis
- [ ] Multi-city support

### Known Limitations

- Requires internet connection for real-time updates
- French language data (stop/line names)
- Limited to TBM network (Bordeaux area)

## 📊 Statistics

- **Lines Supported**: 100+
- **Stops Covered**: 700+
- **Update Frequency**: Every 30 seconds
- **Cache Duration**: 15 days (GTFS static)
- **API Response Time**: ~1-3 seconds

## 🌟 Star History

If you find this project useful, please consider giving it a star! ⭐

---

**Made with ❤️ by [Cyclolysisss](https://github.com/Cyclolysisss)**

*This project is NOT affiliated with TBM or Bordeaux Métropole in ANY WAY.*
