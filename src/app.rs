use crate::config::Config;
use crate::system::{SystemMetrics, Alert, AlertType};
use std::collections::HashMap;
use chrono::Local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuSeverity {
    Moderate,
    High,
    Critical,
}

pub struct CpuSuggestions {
    pub severity: CpuSeverity,
    pub cpu_value: f32,
    pub threshold: f32,
    pub tips: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub enum SuggestionColor {
    Cyan,
    Magenta,
    Yellow,
    Green,
}

pub struct SuggestionCategory {
    pub title: &'static str,
    pub color_hint: SuggestionColor,
    pub tips: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dashboard,
    Alerts,
    Settings,
    Suggestions,
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

    /// Returns contextual suggestions based on current CPU usage severity.
    /// Returns None if CPU is below threshold (no advice needed).
    pub fn get_cpu_suggestions(&self) -> Option<CpuSuggestions> {
        let cpu = match &self.current_metrics {
            Some(m) => m.overall_cpu,
            None => return None,
        };
        let threshold = self.config.thresholds.cpu_percent;

        if cpu < threshold {
            return None;
        }

        // Tier the severity based on how far over threshold we are
        let overage = cpu - threshold;
        let (severity, tips) = if overage >= 30.0 || cpu >= 95.0 {
            (
                CpuSeverity::Critical,
                vec![
                    "Kill runaway processes: open Task Manager (Ctrl+Shift+Esc) and end high-CPU tasks.",
                    "Check for malware or crypto miners consuming cycles unexpectedly.",
                    "Force-close frozen applications that may be spinning in a loop.",
                    "Consider rebooting if the issue persists — memory leaks can cause sustained spikes.",
                    "Check cooling: thermal throttling from overheating can cause erratic CPU spikes.",
                ],
            )
        } else if overage >= 15.0 || cpu >= 80.0 {
            (
                CpuSeverity::High,
                vec![
                    "Identify top consumers in Task Manager and close unnecessary apps.",
                    "Disable startup programs: Task Manager > Startup tab.",
                    "Update or reinstall drivers — faulty drivers can cause CPU spikes.",
                    "Check for Windows Update running in the background (svchost.exe).",
                    "Reduce browser tabs or extensions, they are common CPU hogs.",
                ],
            )
        } else {
            (
                CpuSeverity::Moderate,
                vec![
                    "Monitor usage over time — a brief spike is usually fine.",
                    "Close background apps you are not actively using.",
                    "Check if antivirus is running a scheduled scan.",
                    "Consider increasing the alert threshold if this is your normal workload.",
                ],
            )
        };

        Some(CpuSuggestions {
            severity,
            cpu_value: cpu,
            threshold,
            tips,
        })
    }

    /// Returns all static performance suggestion categories shown on the Suggestions tab.
    pub fn get_all_suggestions() -> Vec<SuggestionCategory> {
        vec![
            SuggestionCategory {
                title: "CPU Performance",
                color_hint: SuggestionColor::Cyan,
                tips: vec![
                    "Close applications you are not actively using — each open app consumes CPU cycles even when idle.",
                    "Disable startup programs that launch automatically: Task Manager > Startup tab, then disable anything you do not need.",
                    "Check for Windows Update or antivirus scans running in the background (visible in Task Manager under svchost.exe or your AV process).",
                    "Reduce browser tabs and extensions — browsers are among the heaviest CPU consumers on most systems.",
                    "Update your device drivers, especially chipset and GPU drivers. Outdated drivers can cause unnecessary CPU overhead.",
                    "If CPU stays high with no obvious cause, run a malware scan — crypto miners and adware are common culprits.",
                    "For sustained high load, check CPU temperatures. Thermal throttling caused by poor cooling reduces effective performance.",
                ],
            },
            SuggestionCategory {
                title: "Memory (RAM)",
                color_hint: SuggestionColor::Magenta,
                tips: vec![
                    "Close unused browser tabs — each tab can consume 100-500 MB of RAM.",
                    "Restart applications that have been running for days. Long-running apps can accumulate memory leaks over time.",
                    "Disable startup programs to free RAM on boot: Task Manager > Startup tab.",
                    "Check for memory-heavy background processes in Task Manager and end tasks that are not needed.",
                    "If swap usage is high, your system is running low on physical RAM. Consider closing apps or upgrading your RAM.",
                    "Avoid running virtual machines or emulators unless necessary — they reserve large blocks of RAM.",
                    "Set a larger pagefile (virtual memory) if your system frequently runs out of RAM under heavy workloads.",
                ],
            },
            SuggestionCategory {
                title: "Network Usage",
                color_hint: SuggestionColor::Yellow,
                tips: vec![
                    "Pause or schedule large downloads and updates for off-peak hours to keep bandwidth available.",
                    "Check for applications uploading data in the background — cloud sync tools (OneDrive, Dropbox, Google Drive) can saturate upload bandwidth.",
                    "Disable auto-play videos in browsers and streaming apps when you only need audio.",
                    "Use Task Manager > Performance > Open Resource Monitor > Network tab to identify which process is consuming bandwidth.",
                    "If upload traffic is unexpectedly high, check for backup software or telemetry processes running silently.",
                    "Restart your router if network speeds are consistently below your plan's rated speed.",
                ],
            },
            SuggestionCategory {
                title: "General System Health",
                color_hint: SuggestionColor::Green,
                tips: vec![
                    "Restart your computer regularly — Windows performs memory cleanup and applies updates on reboot.",
                    "Keep your OS and drivers up to date. Security patches often include performance fixes.",
                    "Run Disk Cleanup (Windows) periodically to remove temporary files that can slow down the system.",
                    "Check your storage drive health using a tool like CrystalDiskInfo — a failing drive causes slowdowns across the whole system.",
                    "Ensure your power plan is set to Balanced or High Performance: Control Panel > Power Options.",
                    "Disable visual effects for better performance: System Properties > Advanced > Performance Settings > Adjust for best performance.",
                    "Scan for malware monthly even if you have an antivirus — some malware disables AV protection.",
                ],
            },
        ]
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

