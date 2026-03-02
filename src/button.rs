use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, Clone, Copy)]
pub enum ButtonEvent {
    ShortPress,
    LongPress,
}

#[cfg(target_os = "linux")]
mod hw {
    use super::*;
    use rppal::gpio::Gpio;
    use std::thread;
    use std::time::{Duration, Instant};

    const PIN_BUTTON: u8 = 17;
    const LONG_PRESS_MS: u128 = 500;
    const POLL_INTERVAL: Duration = Duration::from_millis(10);
    const DEBOUNCE_MS: u128 = 50;

    pub struct ButtonHandler {
        _handle: thread::JoinHandle<()>,
    }

    impl ButtonHandler {
        pub fn new(tx: Sender<ButtonEvent>) -> Result<Self, Box<dyn std::error::Error>> {
            let gpio = Gpio::new()?;
            let pin = gpio.get(PIN_BUTTON)?.into_input_pullup();

            let handle = thread::spawn(move || {
                let _gpio = gpio;
                let mut was_high = pin.is_high();
                let mut press_time: Option<Instant> = None;
                let mut last_edge = Instant::now();

                loop {
                    thread::sleep(POLL_INTERVAL);
                    let is_high = pin.is_high();

                    if is_high == was_high {
                        continue;
                    }

                    let now = Instant::now();
                    if now.duration_since(last_edge).as_millis() < DEBOUNCE_MS {
                        continue;
                    }
                    last_edge = now;

                    if is_high && !was_high {
                        press_time = Some(now);
                    } else if !is_high && was_high {
                        if let Some(t) = press_time.take() {
                            let held = now.duration_since(t).as_millis();
                            let ev = if held >= LONG_PRESS_MS {
                                ButtonEvent::LongPress
                            } else {
                                ButtonEvent::ShortPress
                            };
                            if tx.send(ev).is_err() {
                                break;
                            }
                        }
                    }
                    was_high = is_high;
                }
            });

            Ok(Self { _handle: handle })
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod hw {
    use super::*;
    use std::io::{self, BufRead};
    use std::thread;

    pub struct ButtonHandler {
        _handle: thread::JoinHandle<()>,
    }

    impl ButtonHandler {
        pub fn new(tx: Sender<ButtonEvent>) -> Result<Self, Box<dyn std::error::Error>> {
            println!("[headless] Button: Enter=next, l+Enter=launch, q+Enter=quit");
            let handle = thread::spawn(move || {
                let stdin = io::stdin();
                for line in stdin.lock().lines() {
                    let Ok(line) = line else { break };
                    match line.trim() {
                        "q" | "Q" => break,
                        "l" | "L" => {
                            if tx.send(ButtonEvent::LongPress).is_err() {
                                break;
                            }
                        }
                        _ => {
                            if tx.send(ButtonEvent::ShortPress).is_err() {
                                break;
                            }
                        }
                    }
                }
            });
            Ok(Self { _handle: handle })
        }
    }
}

pub use hw::ButtonHandler;

pub fn create() -> Result<(ButtonHandler, Receiver<ButtonEvent>), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let handler = ButtonHandler::new(tx)?;
    Ok((handler, rx))
}
