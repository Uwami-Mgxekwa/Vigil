# Vigil

Vigil is a lightweight, terminal-based system monitor built with Rust. It gives you real-time visibility into CPU, memory, and network usage directly in your terminal, with configurable alerts when any metric crosses a threshold you define.

---

## Contents

- [Features](#features)
- [Screenshots](#screenshots)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Keyboard Controls](#keyboard-controls)
- [Building from Source](#building-from-source)

---

## Features

- Real-time CPU monitoring — overall usage gauge, per-core breakdown, and usage history sparkline
- CPU suggestions panel — contextual advice appears automatically when CPU usage exceeds your threshold
- Memory monitoring — used/total display, swap usage, and history sparkline
- Network monitoring — live download and upload speeds with history
- Configurable threshold alerts — set your own limits for CPU, memory, and network
- Alert log — every threshold breach is recorded with a timestamp
- Adjustable settings — change thresholds and refresh rate live from inside the app
- Minimal resource overhead — designed to stay out of your way

---

## Screenshots

### Dashboard

The main view. Shows CPU usage with gauge, sparkline history, and per-core breakdown on the left. Memory and network metrics are on the right. Recent alerts are previewed at the bottom.

![Dashboard interface](screenshots/dashboard%20inerface%20.png)

When CPU usage exceeds your configured threshold, a suggestions panel appears at the bottom of the CPU section with actionable advice.

### Alert Logs

A full history of every alert that has been triggered during the current session, with timestamps and the metric value that caused each alert.

![Alert log interface](screenshots/alerts%20log%20interface%20.png)

### Settings

Adjust thresholds and the refresh interval without restarting. Changes take effect immediately for the current session.

![Settings interface](screenshots/settings%20interface%20.png)

### CLI Interface

Vigil can be launched directly from any terminal. The two screenshots below show the CLI startup and initial output.

![CLI interface - step 1](screenshots/screenshot%201%20for%20cli%20interface%20.png)

![CLI interface - step 2](screenshots/Screensh%202%20for%20cli%20interface%20.png)

---

## Installation

### Prerequisites

- Rust (latest stable). Install via [rustup](https://rustup.rs).

### Build and run

```sh
git clone https://github.com/Uwami-Mgxekwa/Vigil.git
cd Vigil
cargo build --release
```

The compiled binary will be at:

```
target/release/Vigil        # Linux / macOS
target/release/Vigil.exe    # Windows
```

Run it directly:

```sh
./target/release/Vigil        # Linux / macOS
.\target\release\Vigil.exe    # Windows
```

---

## Configuration

Vigil reads `config.toml` from the current working directory on startup. If the file is not found, default values are used.

```toml
# How often metrics are refreshed (milliseconds)
refresh_interval = 1000

[thresholds]
# CPU usage percentage that triggers an alert
cpu_percent = 80.0

# Memory usage percentage that triggers an alert
memory_percent = 85.0

# Download speed in MB/s that triggers an alert
network_rx_mb = 10.0

# Upload speed in MB/s that triggers an alert
network_tx_mb = 10.0

# Seconds to wait before repeating the same alert type
alert_cooldown_secs = 10
```

All values can also be adjusted live from the Settings tab inside the app. Those changes apply to the current session only — they are not written back to `config.toml`.

---

## Usage

Start Vigil:

```sh
./target/release/Vigil
```

The app opens in your terminal and begins collecting metrics immediately. Use the tab bar at the top to switch between views.

---

## Keyboard Controls

| Key | Action |
|---|---|
| `Tab` | Cycle through tabs (Dashboard, Alert Logs, Settings) |
| `1` | Go to Dashboard |
| `2` | Go to Alert Logs |
| `3` | Go to Settings |
| `Up` / `Down` | Select a setting (Settings tab) |
| `Left` / `Right` | Decrease / increase the selected setting value |
| `Q` or `Ctrl+C` | Quit |

---

## Building from Source

Vigil targets the stable Rust toolchain. No nightly features are required.

```sh
# Debug build (faster to compile, includes debug symbols)
cargo build

# Release build (optimized, smaller binary)
cargo build --release
```

To target Windows specifically from a Windows machine:

```sh
cargo build --release --target x86_64-pc-windows-msvc
```

---

## License

This project does not currently specify a license. All rights reserved by the author unless stated otherwise.

---

## Author

Developed and maintained by [Uwami Mgxekwa](https://github.com/Uwami-Mgxekwa).
