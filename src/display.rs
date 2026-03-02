pub const LCD_W: u32 = 240;
pub const LCD_H: u32 = 280;

#[cfg(target_os = "linux")]
mod hw {
    use rppal::gpio::{Gpio, OutputPin};
    use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
    use std::thread;
    use std::time::Duration;

    const PIN_DC: u8 = 27;
    const PIN_RST: u8 = 4;
    const CHUNK_SIZE: usize = 4096;
    const Y_OFFSET: u16 = 20;

    pub struct Display {
        spi: Spi,
        dc: OutputPin,
        rst: OutputPin,
    }

    impl Display {
        pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let gpio = Gpio::new()?;
            let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 100_000_000, Mode::Mode0)?;
            let dc = gpio.get(PIN_DC)?.into_output();
            let rst = gpio.get(PIN_RST)?.into_output();
            Ok(Self { spi, dc, rst })
        }

        pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.rst.set_high();
            thread::sleep(Duration::from_millis(100));
            self.rst.set_low();
            thread::sleep(Duration::from_millis(100));
            self.rst.set_high();
            thread::sleep(Duration::from_millis(120));

            self.cmd(0x11, &[]);
            thread::sleep(Duration::from_millis(120));
            self.cmd(0x36, &[0xC0]);
            self.cmd(0x3A, &[0x05]);
            self.cmd(0xB2, &[0x0C, 0x0C, 0x00, 0x33, 0x33]);
            self.cmd(0xB7, &[0x35]);
            self.cmd(0xBB, &[0x32]);
            self.cmd(0xC2, &[0x01]);
            self.cmd(0xC3, &[0x15]);
            self.cmd(0xC4, &[0x20]);
            self.cmd(0xC6, &[0x0F]);
            self.cmd(0xD0, &[0xA4, 0xA1]);
            self.cmd(
                0xE0,
                &[
                    0xD0, 0x08, 0x0E, 0x09, 0x09, 0x05, 0x31, 0x33, 0x48, 0x17, 0x14, 0x15,
                    0x31, 0x34,
                ],
            );
            self.cmd(
                0xE1,
                &[
                    0xD0, 0x08, 0x0E, 0x09, 0x09, 0x15, 0x31, 0x33, 0x48, 0x17, 0x14, 0x15,
                    0x31, 0x34,
                ],
            );
            self.cmd(0x21, &[]);
            self.cmd(0x29, &[]);
            Ok(())
        }

        fn cmd(&mut self, command: u8, data: &[u8]) {
            self.dc.set_low();
            let _ = self.spi.write(&[command]);
            if !data.is_empty() {
                self.dc.set_high();
                let _ = self.spi.write(data);
            }
        }

        fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) {
            let y0 = y0 + Y_OFFSET;
            let y1 = y1 + Y_OFFSET;
            self.cmd(
                0x2A,
                &[(x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8],
            );
            self.cmd(
                0x2B,
                &[(y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8],
            );
            self.cmd(0x2C, &[]);
        }

        pub fn draw_frame(&mut self, data: &[u8]) {
            self.set_window(0, 0, super::LCD_W as u16 - 1, super::LCD_H as u16 - 1);
            self.dc.set_high();
            for chunk in data.chunks(CHUNK_SIZE) {
                let _ = self.spi.write(chunk);
            }
        }

        pub fn fill_black(&mut self) {
            let black = vec![0u8; (super::LCD_W * super::LCD_H * 2) as usize];
            self.draw_frame(&black);
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod hw {
    use std::sync::atomic::{AtomicBool, Ordering};

    static FIRST_FRAME: AtomicBool = AtomicBool::new(true);

    pub struct Display;

    impl Display {
        pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
            println!("[headless] Display created");
            Ok(Self)
        }

        pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            println!(
                "[headless] Display initialized ({}x{})",
                super::LCD_W,
                super::LCD_H
            );
            Ok(())
        }

        pub fn draw_frame(&mut self, data: &[u8]) {
            let w = super::LCD_W;
            let h = super::LCD_H;
            let img = image::ImageBuffer::from_fn(w, h, |x, y| {
                let i = (y * w + x) as usize;
                let raw = ((data[i * 2] as u16) << 8) | data[i * 2 + 1] as u16;
                image::Rgb([
                    (((raw >> 11) & 0x1F) << 3) as u8,
                    (((raw >> 5) & 0x3F) << 2) as u8,
                    ((raw & 0x1F) << 3) as u8,
                ])
            });
            if img.save("preview.png").is_ok() && FIRST_FRAME.swap(false, Ordering::Relaxed) {
                println!("[preview] Saved preview.png — open it to see the UI");
                if cfg!(target_os = "macos") {
                    let _ = std::process::Command::new("open").arg("preview.png").spawn();
                }
            }
        }

        pub fn fill_black(&mut self) {}
    }
}

pub use hw::Display;
