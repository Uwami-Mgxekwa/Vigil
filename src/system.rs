use sysinfo::{System, Networks};
use std::time::Instant;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct CpuCore {
    pub name: String,
    pub usage: f32,
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub overall_cpu: f32,
    pub cores: Vec<CpuCore>,
    pub total_mem_kb: u64,
    pub used_mem_kb: u64,
    pub mem_percent: f32,
    pub total_swap_kb: u64,
    pub used_swap_kb: u64,
    pub rx_bytes_sec: u64,
    pub tx_bytes_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AlertType {
    Cpu,
    Memory,
    NetworkRx,
    NetworkTx,
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertType::Cpu => write!(f, "CPU"),
            AlertType::Memory => write!(f, "Memory"),
            AlertType::NetworkRx => write!(f, "Network RX"),
            AlertType::NetworkTx => write!(f, "Network TX"),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Alert {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub alert_type: AlertType,
    pub message: String,
    pub value: f32,
    pub threshold: f32,
}

pub struct MetricsCollector {
    sys: System,
    networks: Networks,
    last_collection: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let networks = Networks::new_with_refreshed_list();
        Self {
            sys,
            networks,
            last_collection: Instant::now(),
        }
    }

    pub fn collect(&mut self) -> SystemMetrics {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.networks.refresh(true);

        let elapsed = self.last_collection.elapsed().as_secs_f64();
        self.last_collection = Instant::now();

        // Global CPU
        let overall_cpu = self.sys.global_cpu_usage();

        // Individual core CPU usage
        let cores = self.sys.cpus().iter().map(|cpu| CpuCore {
            name: cpu.name().to_string(),
            usage: cpu.cpu_usage(),
        }).collect();

        // Memory
        let total_mem_kb = self.sys.total_memory() / 1024;
        let used_mem = self.sys.used_memory() / 1024;
        let mem_percent = if total_mem_kb > 0 {
            (used_mem as f32 / total_mem_kb as f32) * 100.0
        } else {
            0.0
        };

        let total_swap_kb = self.sys.total_swap() / 1024;
        let used_swap_kb = self.sys.used_swap() / 1024;

        // Network traffic speed
        let mut total_rx = 0;
        let mut total_tx = 0;
        for (_interface_name, network) in &self.networks {
            total_rx += network.received();
            total_tx += network.transmitted();
        }

        // Divide bytes received since last refresh by elapsed seconds
        let rx_bytes_sec = if elapsed > 0.0 {
            (total_rx as f64 / elapsed) as u64
        } else {
            0
        };
        let tx_bytes_sec = if elapsed > 0.0 {
            (total_tx as f64 / elapsed) as u64
        } else {
            0
        };

        SystemMetrics {
            overall_cpu,
            cores,
            total_mem_kb,
            used_mem_kb: used_mem,
            mem_percent,
            total_swap_kb,
            used_swap_kb,
            rx_bytes_sec,
            tx_bytes_sec,
        }
    }
}
