#[cfg(target_os = "linux")]
mod hw {
    use rppal::gpio::{Gpio, OutputPin};

    const PIN_BACKLIGHT: u8 = 22;
    const PIN_RED: u8 = 25;
    const PIN_GREEN: u8 = 24;
    const PIN_BLUE: u8 = 23;

    fn set_pwm(pin: &mut OutputPin, value: u8, freq: f64) {
        if value == 0 {
            let _ = pin.clear_pwm();
            pin.set_high();
        } else {
            let duty = 1.0 - (value as f64 / 255.0);
            let _ = pin.set_pwm_frequency(freq, duty);
        }
    }

    pub struct LedController {
        backlight: OutputPin,
        red: OutputPin,
        green: OutputPin,
        blue: OutputPin,
    }

    impl LedController {
        pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let gpio = Gpio::new()?;
            let mut backlight = gpio.get(PIN_BACKLIGHT)?.into_output();
            let mut red = gpio.get(PIN_RED)?.into_output();
            let mut green = gpio.get(PIN_GREEN)?.into_output();
            let mut blue = gpio.get(PIN_BLUE)?.into_output();

            backlight.set_high();
            red.set_high();
            green.set_high();
            blue.set_high();

            Ok(Self {
                backlight,
                red,
                green,
                blue,
            })
        }

        pub fn set_backlight(&mut self, brightness: u8) {
            if brightness == 0 {
                let _ = self.backlight.clear_pwm();
                self.backlight.set_high();
            } else {
                let duty = 1.0 - (brightness as f64 / 100.0);
                let _ = self.backlight.set_pwm_frequency(1000.0, duty);
            }
        }

        pub fn set_rgb(&mut self, r: u8, g: u8, b: u8) {
            set_pwm(&mut self.red, r, 100.0);
            set_pwm(&mut self.green, g, 100.0);
            set_pwm(&mut self.blue, b, 100.0);
        }

        pub fn off(&mut self) {
            self.set_rgb(0, 0, 0);
            self.set_backlight(0);
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod hw {
    pub struct LedController;

    impl LedController {
        pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
            println!("[headless] LED controller created");
            Ok(Self)
        }

        pub fn set_backlight(&mut self, brightness: u8) {
            println!("[headless] Backlight: {brightness}%");
        }

        pub fn set_rgb(&mut self, r: u8, g: u8, b: u8) {
            println!("[headless] RGB: ({r}, {g}, {b})");
        }

        pub fn off(&mut self) {
            println!("[headless] LEDs off");
        }
    }
}

pub use hw::LedController;
