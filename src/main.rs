mod config;
mod system;
mod app;
mod event;
mod ui;

use std::error::Error;
use std::time::Duration;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use app::{App, ActiveTab};
use config::Config;
use system::MetricsCollector;
use event::{Event, EventHandler};

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Load config
    let config = Config::load_and_merge();

    // 2. Initialize metrics collector
    let mut collector = MetricsCollector::new();

    // 3. Initialize app state
    let mut app = App::new(config);

    // 4. Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 5. Setup event handler
    let mut event_handler = EventHandler::new(Duration::from_millis(app.config.refresh_interval));

    // Fetch initial metrics
    let initial_metrics = collector.collect();
    app.push_metrics(initial_metrics);

    // Draw initial screen
    terminal.draw(|f| ui::draw(f, &mut app))?;

    let mut current_refresh_interval = app.config.refresh_interval;

    // 6. Main event loop
    while !app.should_quit {
        // If refresh interval was changed in settings, reinitialize event handler
        if app.config.refresh_interval != current_refresh_interval {
            current_refresh_interval = app.config.refresh_interval;
            event_handler = EventHandler::new(Duration::from_millis(current_refresh_interval));
        }

        match event_handler.next()? {
            Event::Tick => {
                let metrics = collector.collect();
                app.push_metrics(metrics);
                terminal.draw(|f| ui::draw(f, &mut app))?;
            }
            Event::Key(key) => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Tab => {
                        app.active_tab = match app.active_tab {
                            ActiveTab::Dashboard => ActiveTab::Alerts,
                            ActiveTab::Alerts => ActiveTab::Settings,
                            ActiveTab::Settings => ActiveTab::Suggestions,
                            ActiveTab::Suggestions => ActiveTab::Dashboard,
                        };
                    }
                    KeyCode::Char('1') => {
                        app.active_tab = ActiveTab::Dashboard;
                    }
                    KeyCode::Char('2') => {
                        app.active_tab = ActiveTab::Alerts;
                    }
                    KeyCode::Char('3') => {
                        app.active_tab = ActiveTab::Settings;
                    }
                    KeyCode::Char('4') => {
                        app.active_tab = ActiveTab::Suggestions;
                    }
                    KeyCode::Up => {
                        if app.active_tab == ActiveTab::Settings {
                            if app.selected_setting > 0 {
                                app.selected_setting -= 1;
                            } else {
                                app.selected_setting = 5;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if app.active_tab == ActiveTab::Settings {
                            if app.selected_setting < 5 {
                                app.selected_setting += 1;
                            } else {
                                app.selected_setting = 0;
                            }
                        }
                    }
                    KeyCode::Left => {
                        if app.active_tab == ActiveTab::Settings {
                            app.adjust_setting(false);
                        }
                    }
                    KeyCode::Right => {
                        if app.active_tab == ActiveTab::Settings {
                            app.adjust_setting(true);
                        }
                    }
                    _ => {}
                }
                terminal.draw(|f| ui::draw(f, &mut app))?;
            }
        }
    }

    // 7. Cleanup terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    println!("👁 Vigil has terminated gracefully. Stay alert!");
    Ok(())
}
