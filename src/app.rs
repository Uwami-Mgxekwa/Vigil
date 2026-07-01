use crate::config::Config;
use crate::system::{SystemMetrics, Alert, AlertType};
use std::collections::HashMap;
use chrono::Local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dashboard,
    Alerts,
    Settings,
}

pub struct App {
    pub config: Config,
    pub active_tab: ActiveTab,
    pub cpu_history: Vec<f64>,
    pub mem_history: Vec<f64>,
    pub rx_history: Vec<f64>,
    pub tx_history: Vec<f64>,
    pub current_metrics: Option<SystemMetrics>,
    pub alerts: Vec<Alert>,
    pub last_alert_times: HashMap<AlertType, chrono::DateTime<Local>>,
    pub should_quit: bool,
    // Settings adjustments UI state
    pub selected_setting: usize,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            active_tab: ActiveTab::Dashboard,
            cpu_history: Vec::with_capacity(200),
            mem_history: Vec::with_capacity(200),
            rx_history: Vec::with_capacity(200),
            tx_history: Vec::with_capacity(200),
            current_metrics: None,
            alerts: Vec::new(),
            last_alert_times: HashMap::new(),
            should_quit: false,
            selected_setting: 0,
        }
    }

    pub fn push_metrics(&mut self, metrics: SystemMetrics) {
        // Push CPU
        self.cpu_history.push(metrics.overall_cpu as f64);
        if self.cpu_history.len() > 200 {
            self.cpu_history.remove(0);
        }

        // Push Memory
        self.mem_history.push(metrics.mem_percent as f64);
        if self.mem_history.len() > 200 {
            self.mem_history.remove(0);
        }

        // Push Rx/Tx in MB/s
        let rx_mb = metrics.rx_bytes_sec as f64 / 1024.0 / 1024.0;
        let tx_mb = metrics.tx_bytes_sec as f64 / 1024.0 / 1024.0;
        self.rx_history.push(rx_mb);
        self.tx_history.push(tx_mb);
        if self.rx_history.len() > 200 {
            self.rx_history.remove(0);
        }
        if self.tx_history.len() > 200 {
            self.tx_history.remove(0);
        }

        self.current_metrics = Some(metrics);

        // Check alerts
        self.check_alerts();
    }

    fn check_alerts(&mut self) {
        let metrics = match self.current_metrics.clone() {
            Some(m) => m,
            None => return,
        };

        let now = Local::now();

        // 1. CPU Alert
        let cpu_threshold = self.config.thresholds.cpu_percent;
        if metrics.overall_cpu >= cpu_threshold {
            self.trigger_alert(
                AlertType::Cpu,
                metrics.overall_cpu,
                cpu_threshold,
                format!("CPU usage is high: {:.1}% (threshold: {:.1}%)", metrics.overall_cpu, cpu_threshold),
                now,
            );
        }

        // 2. Memory Alert
        let mem_threshold = self.config.thresholds.memory_percent;
        if metrics.mem_percent >= mem_threshold {
            self.trigger_alert(
                AlertType::Memory,
                metrics.mem_percent,
                mem_threshold,
                format!("Memory usage is high: {:.1}% (threshold: {:.1}%)", metrics.mem_percent, mem_threshold),
                now,
            );
        }

        // 3. Network RX Alert
        let rx_threshold = self.config.thresholds.network_rx_mb;
        let rx_mb = metrics.rx_bytes_sec as f32 / 1024.0 / 1024.0;
        if rx_mb >= rx_threshold {
            self.trigger_alert(
                AlertType::NetworkRx,
                rx_mb,
                rx_threshold,
                format!("Download traffic is high: {:.2} MB/s (threshold: {:.2} MB/s)", rx_mb, rx_threshold),
                now,
            );
        }

        // 4. Network TX Alert
        let tx_threshold = self.config.thresholds.network_tx_mb;
        let tx_mb = metrics.tx_bytes_sec as f32 / 1024.0 / 1024.0;
        if tx_mb >= tx_threshold {
            self.trigger_alert(
                AlertType::NetworkTx,
                tx_mb,
                tx_threshold,
                format!("Upload traffic is high: {:.2} MB/s (threshold: {:.2} MB/s)", tx_mb, tx_threshold),
                now,
            );
        }
    }

    fn trigger_alert(
        &mut self,
        alert_type: AlertType,
        value: f32,
        threshold: f32,
        message: String,
        now: chrono::DateTime<Local>,
    ) {
        let should_trigger = match self.last_alert_times.get(&alert_type) {
            Some(last_time) => {
                let diff_secs = now.signed_duration_since(*last_time).num_seconds();
                diff_secs >= self.config.thresholds.alert_cooldown_secs as i64
            }
            None => true,
        };

        if should_trigger {
            let alert = Alert {
                timestamp: now,
                alert_type: alert_type.clone(),
                message,
                value,
                threshold,
            };
            self.alerts.push(alert);
            if self.alerts.len() > 100 {
                self.alerts.remove(0);
            }
            self.last_alert_times.insert(alert_type, now);
        }
    }

    pub fn adjust_setting(&mut self, increase: bool) {
        let delta = if increase { 1.0 } else { -1.0 };
        match self.selected_setting {
            0 => {
                let step = 100;
                if increase {
                    self.config.refresh_interval = self.config.refresh_interval.saturating_add(step);
                } else {
                    self.config.refresh_interval = self.config.refresh_interval.saturating_sub(step).max(100);
                }
            }
            1 => {
                let step = 5.0;
                let val = self.config.thresholds.cpu_percent + delta * step;
                self.config.thresholds.cpu_percent = val.clamp(0.0, 100.0);
            }
            2 => {
                let step = 5.0;
                let val = self.config.thresholds.memory_percent + delta * step;
                self.config.thresholds.memory_percent = val.clamp(0.0, 100.0);
            }
            3 => {
                let step = 1.0;
                let val = self.config.thresholds.network_rx_mb + delta * step;
                self.config.thresholds.network_rx_mb = val.clamp(0.1, 1000.0);
            }
            4 => {
                let step = 1.0;
                let val = self.config.thresholds.network_tx_mb + delta * step;
                self.config.thresholds.network_tx_mb = val.clamp(0.1, 1000.0);
            }
            5 => {
                let step = 1;
                if increase {
                    self.config.thresholds.alert_cooldown_secs = self.config.thresholds.alert_cooldown_secs.saturating_add(step);
                } else {
                    self.config.thresholds.alert_cooldown_secs = self.config.thresholds.alert_cooldown_secs.saturating_sub(step).max(1);
                }
            }
            _ => {}
        }
    }
}

