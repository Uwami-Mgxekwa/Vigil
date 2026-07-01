use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use clap::Parser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub network_rx_mb: f32,
    pub network_tx_mb: f32,
    pub alert_cooldown_secs: u64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            cpu_percent: 80.0,
            memory_percent: 85.0,
            network_rx_mb: 10.0,
            network_tx_mb: 10.0,
            alert_cooldown_secs: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub refresh_interval: u64,
    pub thresholds: Thresholds,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_interval: 1000,
            thresholds: Thresholds::default(),
        }
    }
}

// CLI Arguments parsed by Clap
#[derive(Parser, Debug)]
#[command(name = "Vigil", author, version, about = "Terminal-based system monitor with alert thresholds", long_about = None)]
pub struct Args {
    /// Path to custom config file
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,

    /// Override polling/refresh interval in ms
    #[arg(short, long)]
    pub interval: Option<u64>,

    /// Override CPU alert threshold percent
    #[arg(long)]
    pub cpu_threshold: Option<f32>,

    /// Override memory alert threshold percent
    #[arg(long)]
    pub mem_threshold: Option<f32>,
}

impl Config {
    pub fn load_and_merge() -> Self {
        let args = Args::parse();
        let path = Path::new(&args.config);

        let mut config = if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match toml::from_str::<Config>(&content) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse config file: {}. Using default configurations.", e);
                        Config::default()
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read config file: {}. Using default configurations.", e);
                    Config::default()
                }
            }
        } else {
            Config::default()
        };

        // Merge CLI argument overrides if present
        if let Some(interval) = args.interval {
            config.refresh_interval = interval;
        }
        if let Some(cpu) = args.cpu_threshold {
            config.thresholds.cpu_percent = cpu;
        }
        if let Some(mem) = args.mem_threshold {
            config.thresholds.memory_percent = mem;
        }

        config
    }
}
