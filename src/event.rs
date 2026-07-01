use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub enum Event {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    receiver: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        
        thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::from_secs(0));

                match event::poll(timeout) {
                    Ok(true) => {
                        match event::read() {
                            Ok(CrosstermEvent::Key(key)) => {
                                // Filter out Release/Repeat events on Windows to prevent double events
                                if key.kind == event::KeyEventKind::Press {
                                    if sender.send(Event::Key(key)).is_err() {
                                        break; // Receiver hung up
                                    }
                                }
                            }
                            Ok(_) => {}
                            Err(_) => {
                                break;
                            }
                        }
                    }
                    Ok(false) => {}
                    Err(_) => {
                        break;
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if sender.send(Event::Tick).is_err() {
                        break; // Receiver hung up
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self { receiver }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.receiver.recv()
    }
}
