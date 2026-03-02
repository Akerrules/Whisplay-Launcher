use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

pub const WIDTH: u32 = 240;
pub const HEIGHT: u32 = 280;
const BUF_SIZE: usize = (WIDTH * HEIGHT * 2) as usize;

pub struct Framebuffer {
    buf: Box<[u8; BUF_SIZE]>,
}

impl Framebuffer {
    pub fn new() -> Self {
        Self {
            buf: Box::new([0u8; BUF_SIZE]),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &*self.buf
    }

    /// Blit RGBA pixels with rounded-corner masking and alpha blending.
    pub fn blit_rgba_rounded(
        &mut self,
        dst_x: i32,
        dst_y: i32,
        w: u32,
        h: u32,
        corner: u32,
        rgba: &[u8],
    ) {
        let iw = w as i32;
        let ih = h as i32;
        let cr = corner as i32;

        for py in 0..ih {
            for px in 0..iw {
                if !inside_rounded_rect(px, py, iw, ih, cr) {
                    continue;
                }

                let src = ((py as u32 * w + px as u32) * 4) as usize;
                if src + 3 >= rgba.len() {
                    continue;
                }
                let (sr, sg, sb, sa) = (rgba[src], rgba[src + 1], rgba[src + 2], rgba[src + 3]);
                if sa == 0 {
                    continue;
                }

                let dx = dst_x + px;
                let dy = dst_y + py;
                if dx < 0 || dy < 0 || dx as u32 >= WIDTH || dy as u32 >= HEIGHT {
                    continue;
                }

                let idx = (dy as usize * WIDTH as usize + dx as usize) * 2;

                if sa == 255 {
                    let v = ((sr as u16 & 0xF8) << 8)
                        | ((sg as u16 & 0xFC) << 3)
                        | (sb as u16 >> 3);
                    self.buf[idx] = (v >> 8) as u8;
                    self.buf[idx + 1] = v as u8;
                } else {
                    let bg = ((self.buf[idx] as u16) << 8) | self.buf[idx + 1] as u16;
                    let br = ((bg >> 11) & 0x1F) as u16;
                    let bgr = ((bg >> 5) & 0x3F) as u16;
                    let bb = (bg & 0x1F) as u16;

                    let a = sa as u16;
                    let ia = 255 - a;
                    let or = ((sr as u16 >> 3) * a + br * ia) / 255;
                    let og = ((sg as u16 >> 2) * a + bgr * ia) / 255;
                    let ob = ((sb as u16 >> 3) * a + bb * ia) / 255;

                    let v = ((or & 0x1F) << 11) | ((og & 0x3F) << 5) | (ob & 0x1F);
                    self.buf[idx] = (v >> 8) as u8;
                    self.buf[idx + 1] = v as u8;
                }
            }
        }
    }
}

fn inside_rounded_rect(px: i32, py: i32, w: i32, h: i32, r: i32) -> bool {
    if r <= 0 {
        return true;
    }
    let in_corner = |cx: i32, cy: i32| -> bool {
        let dx = px - cx;
        let dy = py - cy;
        dx * dx + dy * dy <= r * r
    };
    if px < r && py < r {
        return in_corner(r, r);
    }
    if px >= w - r && py < r {
        return in_corner(w - r - 1, r);
    }
    if px < r && py >= h - r {
        return in_corner(r, h - r - 1);
    }
    if px >= w - r && py >= h - r {
        return in_corner(w - r - 1, h - r - 1);
    }
    true
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            let x = coord.x;
            let y = coord.y;
            if x >= 0 && y >= 0 && (x as u32) < WIDTH && (y as u32) < HEIGHT {
                let raw: u16 = color.into_storage();
                let idx = (y as usize * WIDTH as usize + x as usize) * 2;
                self.buf[idx] = (raw >> 8) as u8;
                self.buf[idx + 1] = raw as u8;
            }
        }
        Ok(())
    }
}
