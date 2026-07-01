use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Sparkline, Tabs},
    Frame,
};
use crate::app::{ActiveTab, App, CpuSeverity};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(10),   // Main Area
            Constraint::Length(1), // Footer / Status Bar
        ])
        .split(f.area());

    // 1. Draw Header & Tabs
    draw_header(f, app, chunks[0]);

    // 2. Draw Main Content depending on active tab
    match app.active_tab {
        ActiveTab::Dashboard => draw_dashboard(f, app, chunks[1]),
        ActiveTab::Alerts => draw_alerts(f, app, chunks[1]),
        ActiveTab::Settings => draw_settings(f, app, chunks[1]),
    }

    // 3. Draw Status bar
    draw_status_bar(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(10)])
        .split(area);

    // Title
    let title_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" 👁  VIGIL ", title_style),
        Span::styled("// SYSTEM MONITOR ", Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    f.render_widget(title, chunks[0]);

    // Tabs
    let tab_titles = vec![" [1] Dashboard ", " [2] Alert Logs ", " [3] Settings "];
    let active_tab_idx = match app.active_tab {
        ActiveTab::Dashboard => 0,
        ActiveTab::Alerts => 1,
        ActiveTab::Settings => 2,
    };
    
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
        .select(active_tab_idx)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[1]);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(75), // Metrics
            Constraint::Percentage(25), // Recent alerts preview
        ])
        .split(area);

    let metrics_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left column: CPU
            Constraint::Percentage(50), // Right column: Mem & Net
        ])
        .split(chunks[0]);

    draw_cpu_panel(f, app, metrics_chunks[0]);
    draw_mem_net_panel(f, app, metrics_chunks[1]);
    draw_alerts_preview(f, app, chunks[1]);
}

fn draw_cpu_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" CPU MONITORING ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let metrics = match &app.current_metrics {
        Some(m) => m,
        None => {
            let paragraph = Paragraph::new("Loading metrics...");
            f.render_widget(paragraph, inner_area);
            return;
        }
    };

    let suggestions = app.get_cpu_suggestions();
    let has_suggestions = suggestions.is_some();

    // Shrink core list when suggestions panel is visible
    let cpu_chunks = if has_suggestions {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Gauge
                Constraint::Length(5), // Sparkline
                Constraint::Min(3),    // Core usages list
                Constraint::Length(7), // Suggestions panel
            ])
            .split(inner_area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Gauge
                Constraint::Length(5), // Sparkline
                Constraint::Min(4),    // Core usages list
                Constraint::Length(0), // No suggestions
            ])
            .split(inner_area)
    };

    // CPU Gauge
    let cpu_val = metrics.overall_cpu;
    let is_warning = cpu_val >= app.config.thresholds.cpu_percent;
    let gauge_color = if is_warning { Color::Red } else { Color::Cyan };

    let cpu_gauge = Gauge::default()
        .block(Block::default().title("Overall Usage"))
        .gauge_style(Style::default().fg(gauge_color).bg(Color::Black))
        .percent(cpu_val.min(100.0) as u16)
        .label(format!("{:.1}%", cpu_val));
    f.render_widget(cpu_gauge, cpu_chunks[0]);

    // CPU Sparkline
    let sparkline_data: Vec<u64> = app.cpu_history.iter().map(|&v| v as u64).collect();
    let cpu_sparkline = Sparkline::default()
        .block(Block::default().title("Usage History (last 200 ticks)"))
        .style(Style::default().fg(gauge_color))
        .data(&sparkline_data);
    f.render_widget(cpu_sparkline, cpu_chunks[1]);

    // Core list
    let core_items: Vec<ListItem> = metrics.cores.iter().map(|core| {
        let is_core_warning = core.usage >= app.config.thresholds.cpu_percent;
        let color = if is_core_warning { Color::LightRed } else { Color::Green };

        let bar_len = ((core.usage / 10.0).min(10.0)) as usize;
        let bar = "█".repeat(bar_len) + &"░".repeat(10 - bar_len);

        ListItem::new(Line::from(vec![
            Span::styled(format!(" {:<8} ", core.name), Style::default().fg(Color::Gray)),
            Span::styled(format!("[{}] ", bar), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:>5.1}%", core.usage), Style::default().fg(color)),
        ]))
    }).collect();

    let core_list = List::new(core_items)
        .block(Block::default().title("CPU Cores").borders(Borders::TOP))
        .style(Style::default().fg(Color::White));
    f.render_widget(core_list, cpu_chunks[2]);

    // Suggestions panel — only rendered when CPU exceeds threshold
    if let Some(s) = suggestions {
        draw_cpu_suggestions(f, &s, cpu_chunks[3]);
    }
}

fn draw_cpu_suggestions(f: &mut Frame, s: &crate::app::CpuSuggestions, area: Rect) {
    let (border_color, severity_label, severity_color) = match s.severity {
        CpuSeverity::Moderate => (Color::Yellow,  "  MODERATE ", Color::Yellow),
        CpuSeverity::High     => (Color::LightRed, "  HIGH     ", Color::LightRed),
        CpuSeverity::Critical => (Color::Red,      "  CRITICAL ", Color::Red),
    };

    let title = format!(
        " SUGGESTIONS [ {} ] CPU at {:.1}% (threshold {:.1}%) ",
        severity_label.trim(), s.cpu_value, s.threshold
    );

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(severity_color).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Show up to 3 tips so they fit comfortably in the 5-line inner area
    let tip_lines: Vec<Line> = s.tips.iter().take(3).enumerate().map(|(i, tip)| {
        Line::from(vec![
            Span::styled(
                format!(" {} ", ["▸", "▸", "▸"][i]),
                Style::default().fg(severity_color),
            ),
            Span::styled(*tip, Style::default().fg(Color::White)),
        ])
    }).collect();

    let paragraph = Paragraph::new(tip_lines);
    f.render_widget(paragraph, inner);
}

fn draw_mem_net_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" MEMORY & NETWORK ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let metrics = match &app.current_metrics {
        Some(m) => m,
        None => return,
    };

    let panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Memory
            Constraint::Percentage(50), // Network
        ])
        .split(inner_area);

    // Memory section
    let mem_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Gauge
            Constraint::Length(5), // Sparkline
            Constraint::Min(1),    // Swap info
        ])
        .split(panel_chunks[0]);

    let is_mem_warning = metrics.mem_percent >= app.config.thresholds.memory_percent;
    let mem_color = if is_mem_warning { Color::Red } else { Color::Magenta };

    let mem_gauge = Gauge::default()
        .block(Block::default().title("Memory Usage"))
        .gauge_style(Style::default().fg(mem_color).bg(Color::Black))
        .percent(metrics.mem_percent.min(100.0) as u16)
        .label(format!("{:.1}% ({:.1} GB / {:.1} GB)", metrics.mem_percent, metrics.used_mem_kb as f32 / 1024.0 / 1024.0, metrics.total_mem_kb as f32 / 1024.0 / 1024.0));
    f.render_widget(mem_gauge, mem_chunks[0]);

    let mem_sparkline_data: Vec<u64> = app.mem_history.iter().map(|&v| v as u64).collect();
    let mem_sparkline = Sparkline::default()
        .block(Block::default().title("Memory History (last 200 ticks)"))
        .style(Style::default().fg(mem_color))
        .data(&mem_sparkline_data);
    f.render_widget(mem_sparkline, mem_chunks[1]);

    // Swap Info
    let swap_percent = if metrics.total_swap_kb > 0 {
        (metrics.used_swap_kb as f32 / metrics.total_swap_kb as f32) * 100.0
    } else {
        0.0
    };
    let swap_paragraph = Paragraph::new(Line::from(vec![
        Span::styled("Swap Usage: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{:.1}% ({:.1} GB / {:.1} GB)", swap_percent, metrics.used_swap_kb as f32 / 1024.0 / 1024.0, metrics.total_swap_kb as f32 / 1024.0 / 1024.0), Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(swap_paragraph, mem_chunks[2]);

    // Network section
    let net_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Current rates
            Constraint::Min(3),    // Sparkline
        ])
        .split(panel_chunks[1]);

    let rx_mb = metrics.rx_bytes_sec as f32 / 1024.0 / 1024.0;
    let tx_mb = metrics.tx_bytes_sec as f32 / 1024.0 / 1024.0;
    let is_rx_warning = rx_mb >= app.config.thresholds.network_rx_mb;
    let is_tx_warning = tx_mb >= app.config.thresholds.network_tx_mb;
    
    let rx_color = if is_rx_warning { Color::Red } else { Color::Yellow };
    let tx_color = if is_tx_warning { Color::Red } else { Color::Blue };

    let rates_line = Line::from(vec![
        Span::styled(" Download (RX): ", Style::default().fg(Color::Gray)),
        Span::styled(format!("{:.2} MB/s ", rx_mb), Style::default().fg(rx_color).add_modifier(Modifier::BOLD)),
        Span::styled(" | Upload (TX): ", Style::default().fg(Color::Gray)),
        Span::styled(format!("{:.2} MB/s", tx_mb), Style::default().fg(tx_color).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(rates_line).block(Block::default().borders(Borders::TOP)), net_chunks[0]);

    // Sparkline for Network (scaled by 10)
    let net_sparkline_data: Vec<u64> = app.rx_history.iter().map(|&v| (v * 10.0) as u64).collect();
    let net_sparkline = Sparkline::default()
        .block(Block::default().title("Download Speed History (RX, 10x scaled)"))
        .style(Style::default().fg(rx_color))
        .data(&net_sparkline_data);
    f.render_widget(net_sparkline, net_chunks[1]);
}

fn draw_alerts_preview(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" RECENT ALERTS ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if app.alerts.is_empty() {
        let p = Paragraph::new("No alerts triggered. System operating normally.").style(Style::default().fg(Color::Green));
        f.render_widget(p, inner_area);
        return;
    }

    let list_items: Vec<ListItem> = app.alerts.iter().rev().take(4).map(|alert| {
        let time_str = alert.timestamp.format("%H:%M:%S").to_string();
        ListItem::new(Line::from(vec![
            Span::styled(format!("[{}] ", time_str), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("[{}] ", alert.alert_type), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(alert.message.clone(), Style::default().fg(Color::LightRed)),
        ]))
    }).collect();

    let list = List::new(list_items);
    f.render_widget(list, inner_area);
}

fn draw_alerts(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" DETAILED ALERT LOGS ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if app.alerts.is_empty() {
        let p = Paragraph::new("No alerts recorded. System operating normally.\n\nAlerts will appear here when thresholds are exceeded.")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);
        f.render_widget(p, inner_area);
        return;
    }

    let list_items: Vec<ListItem> = app.alerts.iter().rev().map(|alert| {
        let date_str = alert.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        ListItem::new(Line::from(vec![
            Span::styled(format!("[{}] ", date_str), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("[{}] ", alert.alert_type), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(alert.message.clone(), Style::default().fg(Color::LightRed)),
        ]))
    }).collect();

    let list = List::new(list_items);
    f.render_widget(list, inner_area);
}

fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" THRESHOLD SETTINGS ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9), // Options list
            Constraint::Min(4),    // Help text
        ])
        .split(inner_area);

    let settings_list = vec![
        ("Refresh Interval (ms)", format!("{} ms", app.config.refresh_interval)),
        ("CPU Threshold (%)", format!("{:.1}%", app.config.thresholds.cpu_percent)),
        ("Memory Threshold (%)", format!("{:.1}%", app.config.thresholds.memory_percent)),
        ("Network RX Threshold (MB/s)", format!("{:.2} MB/s", app.config.thresholds.network_rx_mb)),
        ("Network TX Threshold (MB/s)", format!("{:.2} MB/s", app.config.thresholds.network_tx_mb)),
        ("Alert Cooldown (seconds)", format!("{} s", app.config.thresholds.alert_cooldown_secs)),
    ];

    let items: Vec<ListItem> = settings_list.iter().enumerate().map(|(idx, (name, val))| {
        let is_selected = idx == app.selected_setting;
        let (prefix, name_style, val_style) = if is_selected {
            (" -> ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD), Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED))
        } else {
            ("    ", Style::default().fg(Color::White), Style::default().fg(Color::Cyan))
        };

        ListItem::new(Line::from(vec![
            Span::styled(prefix, name_style),
            Span::styled(format!("{:<30}", name), name_style),
            Span::styled(val, val_style),
        ]))
    }).collect();

    let list = List::new(items)
        .block(Block::default().title("Adjustable Settings"))
        .style(Style::default().fg(Color::White));
    f.render_widget(list, chunks[0]);

    // Help block
    let help_text = vec![
        Line::from(Span::styled("INSTRUCTIONS FOR ADJUSTING SETTINGS:", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" - Use UP / DOWN arrow keys to select a setting.", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(" - Use LEFT / RIGHT arrow keys to decrease / increase threshold values.", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(" - Settings are active immediately for this session.", Style::default().fg(Color::DarkGray))),
    ];
    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::TOP).border_type(BorderType::Rounded).title(" Controls Guide "));
    f.render_widget(help_paragraph, chunks[1]);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let status_text = Line::from(vec![
        Span::styled(" [TAB] Switch View ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::styled(" [Q] Quit ", Style::default().fg(Color::Black).bg(Color::Red)),
        Span::styled(" [Arrow Keys] Settings adjust ", Style::default().fg(Color::Black).bg(Color::Yellow)),
    ]);
    f.render_widget(Paragraph::new(status_text), chunks[0]);

    let alert_count_color = if app.alerts.is_empty() { Color::DarkGray } else { Color::Red };
    let count_text = Line::from(vec![
        Span::styled(format!("Alerts Logged: {} ", app.alerts.len()), Style::default().fg(alert_count_color).add_modifier(Modifier::BOLD)),
    ]).alignment(Alignment::Right);
    f.render_widget(Paragraph::new(count_text), chunks[1]);
}
