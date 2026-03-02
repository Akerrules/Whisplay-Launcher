mod apps;
mod button;
mod display;
mod framebuffer;
mod led;
mod menu;
mod status;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use button::{ButtonEvent, ButtonHandler};
use display::Display;
use framebuffer::Framebuffer;
use led::LedController;

const ACCENT: [u8; 3] = [29, 185, 84];

struct Hardware {
    display: Display,
    led: LedController,
    _button: ButtonHandler,
    button_rx: Receiver<ButtonEvent>,
}

impl Hardware {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut display = Display::new()?;
        display.init()?;

        let mut led = LedController::new()?;
        led.set_backlight(70);
        led.set_rgb(ACCENT[0], ACCENT[1], ACCENT[2]);

        let (_button, button_rx) = button::create()?;

        Ok(Self {
            display,
            led,
            _button,
            button_rx,
        })
    }
}

impl Drop for Hardware {
    fn drop(&mut self) {
        self.display.fill_black();
        self.led.off();
    }
}

fn base_dir() -> std::path::PathBuf {
    // Prefer CWD (matches systemd WorkingDirectory and normal usage).
    // Fall back to the binary's own directory for standalone deployment.
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("apps.json").exists() {
        return cwd;
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or(cwd)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&running))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&running))?;

    let dir = base_dir();

    println!("Whisplay Launcher (Rust)");
    println!("{}", "=".repeat(40));

    println!("Discovering apps:");
    let apps = apps::load_apps(&dir);
    println!("\n{} app(s) loaded", apps.len());
    println!("Short press = browse  |  Long press = launch");
    println!("{}\n", "=".repeat(40));

    let mut hw = Hardware::new()?;
    let mut fb = Framebuffer::new();
    let mut selected: usize = 0;
    let mut needs_redraw = true;

    let mut last_time = status::local_time();
    let mut last_wifi = status::wifi_state();
    let mut last_status_check = Instant::now();

    while running.load(Ordering::Relaxed) {
        if last_status_check.elapsed() >= Duration::from_secs(1) {
            let time_str = status::local_time();
            let wifi = status::wifi_state();
            if time_str != last_time || wifi != last_wifi {
                last_time = time_str;
                last_wifi = wifi;
                needs_redraw = true;
            }
            last_status_check = Instant::now();
        }

        if needs_redraw {
            menu::render(&mut fb, &apps, selected, &last_time, &last_wifi);
            hw.display.draw_frame(fb.as_bytes());
            needs_redraw = false;
            hw.led.set_rgb(ACCENT[0], ACCENT[1], ACCENT[2]);
        }

        // Wait for at least one event (or timeout)
        let first = hw.button_rx.recv_timeout(Duration::from_millis(50));

        // Drain any queued events so rapid presses collapse into one redraw
        let mut launch = false;
        let mut disconnected = false;

        let mut handle = |evt| match evt {
            ButtonEvent::ShortPress => {
                if !apps.is_empty() {
                    selected = (selected + 1) % apps.len();
                    needs_redraw = true;
                }
            }
            ButtonEvent::LongPress => {
                launch = true;
            }
        };

        match first {
            Ok(evt) => handle(evt),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => { disconnected = true; }
        }

        while let Ok(evt) = hw.button_rx.try_recv() {
            handle(evt);
        }

        if disconnected {
            break;
        }

        if launch && !apps.is_empty() {
            let app = &apps[selected];
            let [r, g, b] = app.accent_color();

            menu::render_splash(&mut fb, app);
            hw.display.draw_frame(fb.as_bytes());
            hw.led.set_rgb(r, g, b);
            thread::sleep(Duration::from_millis(600));

            drop(hw);

            apps::launch(app, &dir);

            thread::sleep(Duration::from_millis(300));
            hw = Hardware::new()?;
            needs_redraw = true;
            println!("Returned to launcher menu\n");
        } else if needs_redraw {
            hw.led.set_rgb(0, 100, 255);
        }
    }

    println!("\nLauncher shutting down...");
    Ok(())
}
