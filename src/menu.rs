use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle, Rectangle, RoundedRectangle};

use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};
use u8g2_fonts::{fonts, FontRenderer};

use crate::apps::{AppConfig, IconData};
use crate::framebuffer::Framebuffer;
use crate::status::WifiState;

const W: i32 = 240;

// Grid layout (PiSugar home-screen style):
//
//  y=0..24    Status bar: [time] [wifi]
//  y=36..276  2×2 app grid (2 rows × 120px)
//  y=276..280 Page dots

const STATUS_H: i32 = 24;
const GRID_Y: i32 = 32;
const GRID_COLS: usize = 2;
const GRID_ROWS: usize = 2;
const ITEMS_PER_PAGE: usize = GRID_COLS * GRID_ROWS;
const COL_W: i32 = W / GRID_COLS as i32;
const COL_INSET: i32 = 10;
const ICON_SZ: u32 = 80;
const ICON_CORNER: u32 = 20;
const ROW_H: i32 = 116;
const DOTS_Y: i32 = GRID_Y + GRID_ROWS as i32 * ROW_H;

const SPLASH_SZ: u32 = 96;
const SPLASH_CORNER: u32 = 22;

fn rgb(r: u8, g: u8, b: u8) -> Rgb565 {
    Rgb565::new(r >> 3, g >> 2, b >> 3)
}

fn bg() -> Rgb565 {
    rgb(0, 0, 0)
}

fn text_secondary() -> Rgb565 {
    rgb(142, 142, 147)
}

fn dim_icon_fill() -> Rgb565 {
    rgb(44, 44, 46)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn render(fb: &mut Framebuffer, apps: &[AppConfig], selected: usize, time_str: &str, wifi: &WifiState) {
    fb.clear(bg()).ok();

    if apps.is_empty() {
        render_no_apps(fb, time_str, wifi);
        return;
    }

    render_status_bar(fb, time_str, wifi);

    let page = selected / ITEMS_PER_PAGE;
    let page_start = page * ITEMS_PER_PAGE;

    for slot in 0..ITEMS_PER_PAGE {
        let idx = page_start + slot;
        if idx >= apps.len() {
            break;
        }
        let row = slot / GRID_COLS;
        let col = slot % GRID_COLS;
        draw_grid_cell(fb, &apps[idx], row, col, idx == selected);
    }

    let total_pages = (apps.len() + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE;
    if total_pages > 1 {
        let [ar, ag, ab] = apps[selected].accent_color();
        draw_page_dots(fb, page, total_pages, rgb(ar, ag, ab));
    }
}

pub fn render_splash(fb: &mut Framebuffer, app: &AppConfig) {
    fb.clear(bg()).ok();

    let [r, g, b] = app.accent_color();
    let accent = rgb(r, g, b);

    let ix = (W - SPLASH_SZ as i32) / 2;
    let iy = 60;
    draw_icon(fb, ix, iy, SPLASH_SZ, SPLASH_CORNER, accent, app.icon_type(), true, app.icon_data.as_ref());

    let text = format!("Launching {}...", app.name);
    let font = FontRenderer::new::<fonts::u8g2_font_helvB18_tr>();
    let _ = font.render_aligned(
        text.as_str(),
        Point::new(W / 2, iy as i32 + SPLASH_SZ as i32 + 28),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(Rgb565::WHITE),
        fb,
    );
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn render_status_bar(fb: &mut Framebuffer, time_str: &str, wifi: &WifiState) {
    let font = FontRenderer::new::<fonts::u8g2_font_helvB12_tr>();
    let _ = font.render_aligned(
        time_str,
        Point::new(18, STATUS_H / 2),
        VerticalPosition::Center,
        HorizontalAlignment::Left,
        FontColor::Transparent(Rgb565::WHITE),
        fb,
    );

    draw_wifi_icon(fb, 58, wifi);
}

fn render_no_apps(fb: &mut Framebuffer, time_str: &str, wifi: &WifiState) {
    render_status_bar(fb, time_str, wifi);

    let font = FontRenderer::new::<fonts::u8g2_font_helvB18_tr>();
    let _ = font.render_aligned(
        "No apps found",
        Point::new(W / 2, 130),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(text_secondary()),
        fb,
    );

    let small = FontRenderer::new::<fonts::u8g2_font_helvR14_tr>();
    let _ = small.render_aligned(
        "Edit apps.json",
        Point::new(W / 2, 160),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(rgb(72, 72, 74)),
        fb,
    );
}

// ---------------------------------------------------------------------------
// Grid cell: icon tile + name underneath
// ---------------------------------------------------------------------------

fn draw_grid_cell(fb: &mut Framebuffer, app: &AppConfig, row: usize, col: usize, selected: bool) {
    let [ar, ag, ab] = app.accent_color();
    let accent = rgb(ar, ag, ab);

    let cell_x = col as i32 * COL_W;
    let cell_y = GRID_Y + row as i32 * ROW_H;

    let inset = if col == 0 { COL_INSET } else { -COL_INSET };
    let icon_x = cell_x + (COL_W - ICON_SZ as i32) / 2 + inset;
    let icon_y = cell_y;

    if selected {
        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(icon_x - 4, icon_y - 4),
                Size::new(ICON_SZ + 8, ICON_SZ + 8),
            ),
            Size::new(ICON_CORNER + 3, ICON_CORNER + 3),
        )
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 3))
        .draw(fb)
        .ok();
    }

    draw_icon(
        fb, icon_x, icon_y, ICON_SZ, ICON_CORNER,
        accent, app.icon_type(), true, app.icon_data.as_ref(),
    );

    let name_y = icon_y + ICON_SZ as i32 + 14;
    let name_x = cell_x + COL_W / 2;
    let text_color = if selected { Rgb565::WHITE } else { rgb(200, 200, 204) };

    let font = FontRenderer::new::<fonts::u8g2_font_helvB12_tr>();
    let _ = font.render_aligned(
        app.name.as_str(),
        Point::new(name_x, name_y),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(text_color),
        fb,
    );
}

// ---------------------------------------------------------------------------
// Page dots
// ---------------------------------------------------------------------------

fn draw_page_dots(fb: &mut Framebuffer, current_page: usize, total_pages: usize, accent: Rgb565) {
    let dot_d: u32 = 8;
    let gap: i32 = 14;
    let total_w = total_pages as i32 * dot_d as i32 + (total_pages as i32 - 1) * (gap - dot_d as i32);
    let start_x = (W - total_w) / 2;
    let cy = DOTS_Y + 10;

    for i in 0..total_pages {
        let x = start_x + i as i32 * gap;
        let color = if i == current_page { accent } else { rgb(50, 50, 54) };
        Circle::new(Point::new(x, cy - dot_d as i32 / 2), dot_d)
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(fb)
            .ok();
    }
}

// ---------------------------------------------------------------------------
// Squircle icon — size-independent, glyphs scale automatically
// ---------------------------------------------------------------------------

fn draw_icon(
    fb: &mut Framebuffer,
    x: i32,
    y: i32,
    size: u32,
    corner: u32,
    accent: Rgb565,
    icon_type: &str,
    selected: bool,
    icon_data: Option<&IconData>,
) {
    if let Some(data) = icon_data {
        let (rgba, src_size) = if size <= 64 {
            (&data.rgba_64, 64u32)
        } else if size <= 80 {
            (&data.rgba_80, 80u32)
        } else {
            (&data.rgba_96, 96u32)
        };
        fb.blit_rgba_rounded(x, y, src_size, src_size, corner, rgba);
        return;
    }

    let fill_color = if selected { accent } else { dim_icon_fill() };

    RoundedRectangle::with_equal_corners(
        Rectangle::new(Point::new(x, y), Size::new(size, size)),
        Size::new(corner, corner),
    )
    .into_styled(PrimitiveStyle::with_fill(fill_color))
    .draw(fb)
    .ok();

    let cx = x + size as i32 / 2;
    let cy = y + size as i32 / 2;
    let s = size as f32 / 40.0;
    let glyph_c = if selected { bg() } else { accent };

    match icon_type {
        "music" => glyph_music(fb, cx, cy, glyph_c, s),
        "bird" => glyph_bird(fb, cx, cy, glyph_c, selected, s),
        "game" => glyph_game(fb, cx, cy, glyph_c, selected, s),
        "settings" => glyph_settings(fb, cx, cy, glyph_c, selected, s),
        _ => glyph_default(fb, cx, cy, glyph_c, s),
    }
}

fn sc(val: i32, s: f32) -> i32 {
    (val as f32 * s) as i32
}

fn scu(val: u32, s: f32) -> u32 {
    ((val as f32 * s) as u32).max(1)
}

fn glyph_music(fb: &mut Framebuffer, cx: i32, cy: i32, c: Rgb565, s: f32) {
    let fill = PrimitiveStyle::with_fill(c);
    let stem = PrimitiveStyle::with_stroke(c, scu(2, s));

    Circle::new(Point::new(cx - sc(8, s), cy + sc(2, s)), scu(9, s))
        .into_styled(fill)
        .draw(fb)
        .ok();
    Circle::new(Point::new(cx + sc(1, s), cy), scu(9, s))
        .into_styled(fill)
        .draw(fb)
        .ok();

    Line::new(
        Point::new(cx, cy + sc(4, s)),
        Point::new(cx, cy - sc(10, s)),
    )
    .into_styled(stem)
    .draw(fb)
    .ok();
    Line::new(
        Point::new(cx + sc(9, s), cy + sc(2, s)),
        Point::new(cx + sc(9, s), cy - sc(12, s)),
    )
    .into_styled(stem)
    .draw(fb)
    .ok();
    Line::new(
        Point::new(cx, cy - sc(10, s)),
        Point::new(cx + sc(9, s), cy - sc(12, s)),
    )
    .into_styled(PrimitiveStyle::with_stroke(c, scu(3, s)))
    .draw(fb)
    .ok();
}

fn glyph_bird(fb: &mut Framebuffer, cx: i32, cy: i32, c: Rgb565, selected: bool, s: f32) {
    let fill = PrimitiveStyle::with_fill(c);

    Circle::new(Point::new(cx - sc(9, s), cy - sc(3, s)), scu(18, s))
        .into_styled(fill)
        .draw(fb)
        .ok();
    Circle::new(Point::new(cx + sc(1, s), cy - sc(11, s)), scu(12, s))
        .into_styled(fill)
        .draw(fb)
        .ok();

    let beak = if selected { rgb(200, 140, 40) } else { rgb(255, 180, 50) };
    Rectangle::new(
        Point::new(cx + sc(10, s), cy - sc(8, s)),
        Size::new(scu(5, s), scu(3, s)),
    )
    .into_styled(PrimitiveStyle::with_fill(beak))
    .draw(fb)
    .ok();

    let eye_c = if selected { rgb(40, 40, 40) } else { Rgb565::WHITE };
    Circle::new(Point::new(cx + sc(5, s), cy - sc(9, s)), scu(3, s))
        .into_styled(PrimitiveStyle::with_fill(eye_c))
        .draw(fb)
        .ok();
}

fn glyph_game(fb: &mut Framebuffer, cx: i32, cy: i32, c: Rgb565, selected: bool, s: f32) {
    let fill = PrimitiveStyle::with_fill(c);
    let btn_c = if selected { bg() } else { dim_icon_fill() };
    let btn = PrimitiveStyle::with_fill(btn_c);

    RoundedRectangle::with_equal_corners(
        Rectangle::new(
            Point::new(cx - sc(14, s), cy - sc(7, s)),
            Size::new(scu(28, s), scu(14, s)),
        ),
        Size::new(scu(4, s), scu(4, s)),
    )
    .into_styled(fill)
    .draw(fb)
    .ok();
    Circle::new(Point::new(cx - sc(16, s), cy - sc(6, s)), scu(12, s))
        .into_styled(fill)
        .draw(fb)
        .ok();
    Circle::new(Point::new(cx + sc(4, s), cy - sc(6, s)), scu(12, s))
        .into_styled(fill)
        .draw(fb)
        .ok();

    Rectangle::new(
        Point::new(cx - sc(11, s), cy - sc(2, s)),
        Size::new(scu(6, s), scu(2, s)),
    )
    .into_styled(btn)
    .draw(fb)
    .ok();
    Rectangle::new(
        Point::new(cx - sc(10, s), cy - sc(3, s)),
        Size::new(scu(2, s), scu(5, s)),
    )
    .into_styled(btn)
    .draw(fb)
    .ok();

    Circle::new(Point::new(cx + sc(5, s), cy - sc(3, s)), scu(3, s))
        .into_styled(btn)
        .draw(fb)
        .ok();
    Circle::new(Point::new(cx + sc(9, s), cy - sc(1, s)), scu(3, s))
        .into_styled(btn)
        .draw(fb)
        .ok();
}

fn glyph_settings(fb: &mut Framebuffer, cx: i32, cy: i32, c: Rgb565, selected: bool, s: f32) {
    let fill = PrimitiveStyle::with_fill(c);

    let teeth = 6;
    let outer_r = 12.0 * s;
    for i in 0..teeth {
        let angle = (i as f32) * std::f32::consts::TAU / teeth as f32;
        let tx = cx + (outer_r * angle.cos()) as i32;
        let ty = cy + (outer_r * angle.sin()) as i32;
        Circle::new(Point::new(tx - sc(3, s), ty - sc(3, s)), scu(7, s))
            .into_styled(fill)
            .draw(fb)
            .ok();
    }

    Circle::new(Point::new(cx - sc(8, s), cy - sc(8, s)), scu(16, s))
        .into_styled(fill)
        .draw(fb)
        .ok();

    let inner_c = if selected { bg() } else { dim_icon_fill() };
    Circle::new(Point::new(cx - sc(4, s), cy - sc(4, s)), scu(8, s))
        .into_styled(PrimitiveStyle::with_fill(inner_c))
        .draw(fb)
        .ok();
}

fn glyph_default(fb: &mut Framebuffer, cx: i32, cy: i32, c: Rgb565, s: f32) {
    let fill = PrimitiveStyle::with_fill(c);
    for dx in [-7i32, 7] {
        for dy in [-7i32, 7] {
            Circle::new(
                Point::new(cx + sc(dx, s) - sc(3, s), cy + sc(dy, s) - sc(3, s)),
                scu(7, s),
            )
            .into_styled(fill)
            .draw(fb)
            .ok();
        }
    }
}

// ---------------------------------------------------------------------------
// WiFi status icon — three concentric 90° arcs + base dot
// ---------------------------------------------------------------------------

fn draw_wifi_icon(fb: &mut Framebuffer, x: i32, state: &WifiState) {
    let color = match state {
        WifiState::Connected => rgb(29, 185, 84),
        WifiState::Disconnected => rgb(255, 69, 58),
        WifiState::Unknown => rgb(100, 100, 104),
    };

    let cx = x + 8;
    let cy = 15;

    for &r in &[4i32, 7, 10] {
        draw_wifi_arc(fb, cx, cy, r, color);
    }

    Circle::new(Point::new(cx - 1, cy - 2), 3)
        .into_styled(PrimitiveStyle::with_fill(color))
        .draw(fb)
        .ok();
}

fn draw_wifi_arc(fb: &mut Framebuffer, cx: i32, cy: i32, r: i32, color: Rgb565) {
    let x_limit = (r as f32 * 0.707) as i32;
    for ri in r..=r + 1 {
        let r_sq = ri * ri;
        for dx in -x_limit..=x_limit {
            let dy_sq = r_sq - dx * dx;
            if dy_sq < 0 {
                continue;
            }
            let dy = (dy_sq as f32).sqrt() as i32;
            Pixel(Point::new(cx + dx, cy - dy), color)
                .draw(fb)
                .ok();
        }
    }
}
