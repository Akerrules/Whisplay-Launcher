#!/usr/bin/env python3
"""
Whisplay Settings — adjust display brightness and system volume.

Controls:
  Short press  → cycle through settings / increase value
  Long press   → toggle between menu and adjust mode
  3× rapid     → exit back to launcher
"""

import sys
import os
import time
import subprocess
import threading

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(__file__))
from whisplay_hw import WhisPlayBoard, LCD_W, LCD_H

# -- Appearance ---------------------------------------------------------------

BG = (0, 0, 0)
ACCENT = (100, 120, 255)
DIM = (60, 60, 64)
TEXT_PRIMARY = (255, 255, 255)
TEXT_SECONDARY = (142, 142, 147)
BAR_BG = (38, 38, 40)
BAR_FILL_ACTIVE = ACCENT
BAR_FILL_IDLE = (80, 80, 84)

TITLE_Y = 8
ROW_START_Y = 60
ROW_H = 90
BAR_X = 30
BAR_W = 180
BAR_H = 14
BAR_CORNER = 7

# -- System helpers -----------------------------------------------------------

def _audio_env():
    """Build env dict so PipeWire/PulseAudio work even when running as root."""
    env = os.environ.copy()
    if os.getuid() == 0 and "XDG_RUNTIME_DIR" not in env:
        for uid in (1000, 1001):
            rt = f"/run/user/{uid}"
            if os.path.isdir(rt):
                env["XDG_RUNTIME_DIR"] = rt
                break
    return env


def _run(cmd, capture=False):
    """Run a command with the audio-aware environment."""
    env = _audio_env()
    if capture:
        return subprocess.check_output(
            cmd, stderr=subprocess.DEVNULL, text=True, env=env,
        )
    subprocess.run(
        cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env,
    )


def _detect_audio_backend():
    """Detect which audio system is active: 'pipewire', 'pulse', or 'alsa'."""
    try:
        _run(["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"], capture=True)
        return "pipewire"
    except Exception:
        pass
    try:
        _run(["pactl", "get-sink-volume", "@DEFAULT_SINK@"], capture=True)
        return "pulse"
    except Exception:
        pass
    return "alsa"


def _find_alsa_control():
    """Return the first ALSA simple-mixer control that exists."""
    for name in ("Master", "Speaker", "Headphone", "Playback", "PCM"):
        try:
            _run(["amixer", "sget", name], capture=True)
            return name
        except Exception:
            continue
    return "Master"


_audio_backend = _detect_audio_backend()
_alsa_control = _find_alsa_control() if _audio_backend == "alsa" else None


def get_volume():
    """Read current system volume (0..100)."""
    try:
        if _audio_backend == "pipewire":
            out = _run(
                ["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"], capture=True,
            )
            parts = out.strip().split()
            if len(parts) >= 2:
                return max(0, min(100, round(float(parts[1]) * 100)))
        elif _audio_backend == "pulse":
            out = _run(
                ["pactl", "get-sink-volume", "@DEFAULT_SINK@"], capture=True,
            )
            for token in out.split():
                if token.endswith("%"):
                    return max(0, min(100, int(token[:-1])))
        else:
            out = _run(["amixer", "sget", _alsa_control], capture=True)
            for line in out.splitlines():
                if "%" in line:
                    start = line.index("[") + 1
                    end = line.index("%")
                    return int(line[start:end])
    except Exception:
        pass
    return 50


def set_volume(pct):
    """Set system volume (0..100)."""
    pct = max(0, min(100, pct))
    try:
        if _audio_backend == "pipewire":
            _run(["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", f"{pct}%"])
        elif _audio_backend == "pulse":
            _run(["pactl", "set-sink-volume", "@DEFAULT_SINK@", f"{pct}%"])
        else:
            _run(["amixer", "sset", _alsa_control, f"{pct}%"])
    except Exception:
        pass


# -- Drawing ------------------------------------------------------------------

def try_load_font(size):
    paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSansBold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
    ]
    for p in paths:
        if os.path.isfile(p):
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


font_title = try_load_font(20)
font_label = try_load_font(16)
font_value = try_load_font(14)


def draw_rounded_rect(draw, xy, corner, fill):
    x0, y0, x1, y1 = xy
    r = corner
    draw.rectangle([x0 + r, y0, x1 - r, y1], fill=fill)
    draw.rectangle([x0, y0 + r, x1, y1 - r], fill=fill)
    draw.pieslice([x0, y0, x0 + 2 * r, y0 + 2 * r], 180, 270, fill=fill)
    draw.pieslice([x1 - 2 * r, y0, x1, y0 + 2 * r], 270, 360, fill=fill)
    draw.pieslice([x0, y1 - 2 * r, x0 + 2 * r, y1], 90, 180, fill=fill)
    draw.pieslice([x1 - 2 * r, y1 - 2 * r, x1, y1], 0, 90, fill=fill)


def render(board, cursor, editing, brightness, volume):
    img = Image.new("RGB", (LCD_W, LCD_H), BG)
    draw = ImageDraw.Draw(img)

    # Title
    draw.text((LCD_W // 2, TITLE_Y), "Settings", fill=TEXT_PRIMARY,
              font=font_title, anchor="mt")

    # Accent underline
    draw.line([(90, 34), (150, 34)], fill=ACCENT, width=1)

    settings = [
        ("Brightness", brightness, "%"),
        ("Volume", volume, "%"),
    ]

    for i, (label, value, unit) in enumerate(settings):
        y = ROW_START_Y + i * ROW_H
        selected = i == cursor
        active = selected and editing

        # Row background for selected item
        if selected:
            draw_rounded_rect(draw, (10, y - 6, LCD_W - 10, y + ROW_H - 16), 12,
                              fill=(28, 28, 30))
            draw.rectangle((10, y - 2, 13, y + ROW_H - 20), fill=ACCENT)

        label_color = TEXT_PRIMARY if selected else TEXT_SECONDARY
        draw.text((BAR_X, y), label, fill=label_color, font=font_label, anchor="lt")

        val_text = f"{value}{unit}"
        val_color = ACCENT if active else (TEXT_PRIMARY if selected else TEXT_SECONDARY)
        draw.text((LCD_W - BAR_X, y), val_text, fill=val_color,
                  font=font_value, anchor="rt")

        # Progress bar
        bar_y = y + 30
        draw_rounded_rect(draw, (BAR_X, bar_y, BAR_X + BAR_W, bar_y + BAR_H),
                          BAR_CORNER, fill=BAR_BG)

        fill_w = int(BAR_W * value / 100)
        if fill_w > 0:
            bar_color = BAR_FILL_ACTIVE if active else (ACCENT if selected else BAR_FILL_IDLE)
            draw_rounded_rect(
                draw,
                (BAR_X, bar_y, BAR_X + max(fill_w, BAR_H), bar_y + BAR_H),
                BAR_CORNER, fill=bar_color,
            )

    # Hint
    if editing:
        hint = "press \u00b7 adjust    hold \u00b7 done"
    else:
        hint = "press \u00b7 select    hold \u00b7 edit"
    draw.text((LCD_W // 2, LCD_H - 16), hint,
              fill=(72, 72, 74), font=font_value, anchor="mm")

    board.display_image(img)


# -- Main loop ----------------------------------------------------------------

def main():
    board = WhisPlayBoard(backlight=70)
    board.set_rgb(*ACCENT)

    brightness = board.get_backlight()
    volume = get_volume()
    cursor = 0
    editing = False
    needs_redraw = True

    press_time = None
    press_times = []
    lock = threading.Lock()
    event_queue = []
    running = True

    LONG_PRESS_MS = 500
    TRIPLE_WINDOW = 1.5

    def on_press():
        nonlocal press_time
        press_time = time.time()

    def on_release():
        nonlocal press_time, running
        if press_time is None:
            return

        held = (time.time() - press_time) * 1000
        press_time = None

        now = time.time()
        press_times.append(now)
        recent = [t for t in press_times if now - t <= TRIPLE_WINDOW]
        press_times.clear()
        press_times.extend(recent)
        if len(press_times) >= 3:
            press_times.clear()
            with lock:
                event_queue.append("exit")
            return

        ev = "long" if held >= LONG_PRESS_MS else "short"
        with lock:
            event_queue.append(ev)

    board.on_button_press(on_press)
    board.on_button_release(on_release)

    try:
        while running:
            if needs_redraw:
                render(board, cursor, editing, brightness, volume)
                needs_redraw = False

            with lock:
                events = list(event_queue)
                event_queue.clear()

            for ev in events:
                if ev == "exit":
                    running = False
                    break
                elif ev == "short":
                    if editing:
                        if cursor == 0:
                            brightness = (brightness + 10) % 110
                            if brightness > 100:
                                brightness = 0
                            board.set_backlight(brightness)
                        else:
                            volume = (volume + 10) % 110
                            if volume > 100:
                                volume = 0
                            set_volume(volume)
                    else:
                        cursor = (cursor + 1) % 2
                    needs_redraw = True
                elif ev == "long":
                    editing = not editing
                    needs_redraw = True

            time.sleep(0.05)

    except KeyboardInterrupt:
        pass
    finally:
        board.cleanup()


if __name__ == "__main__":
    main()
