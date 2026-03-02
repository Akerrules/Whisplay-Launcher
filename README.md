# Whisplay Launcher

A visual app launcher for the PiSugar Whisplay HAT. Browse and launch apps from a single menu on the 240x280 LCD using the HAT's button.

## Features

- **Carousel menu** — cycle through installed apps with short presses, launch with a long press
- **Programmatic icons** — each app gets a drawn icon (music note, bird, gamepad, etc.) — no image files needed
- **Subprocess isolation** — apps run as independent processes; crashes don't take down the launcher
- **Triple-press exit** — press the button three times quickly inside any app to return to the launcher
- **Auto-discovery** — register apps in `apps.json`; the launcher validates paths at startup
- **LED feedback** — green while browsing, blue flash on cycle, app accent color on launch

## Hardware

- Raspberry Pi (tested on Pi Zero 2 W)
- PiSugar Whisplay HAT with driver installed
- Internet connection (for apps that need it)

## Directory Layout (on Pi)

```
~/Whisplay/example/
├── Whisplay-Launcher/      ← this repo
│   ├── launcher.py
│   ├── apps.json
│   ├── exit_helper.py
│   └── ...
├── Whisplay-Spotify-app/   ← app repo
│   └── spotify_player.py
├── Whisplay-FlappyBird/    ← app repo
│   └── flappy_bird.py
└── (future apps)/
```

## Setup

### 1. Install the Whisplay HAT driver (if not done already)

```bash
git clone https://github.com/PiSugar/Whisplay.git --depth 1
cd Whisplay/Driver
sudo bash install_wm8960_drive.sh
sudo reboot
```

### 2. Clone the launcher and your apps

```bash
cd ~/Whisplay/example
git clone https://github.com/Akerrules/Whisplay-Launcher.git
git clone https://github.com/Akerrules/Whisplay-Spotify-app.git
git clone https://github.com/Akerrules/Whisplay-FlappyBird.git
```

### 3. Install dependencies

```bash
cd Whisplay-Launcher
pip install -r requirements.txt
```

Each app may have its own dependencies — see its own README.

### 4. Configure apps.json

The default `apps.json` points to sibling directories. Adjust paths if your layout differs:

```json
[
  {
    "name": "Spotify Player",
    "script": "../Whisplay-Spotify-app/spotify_player.py",
    "description": "Vinyl album art + playback control",
    "icon": "music",
    "color": [29, 185, 84]
  },
  {
    "name": "Flappy Bird",
    "script": "../Whisplay-FlappyBird/flappy_bird.py",
    "description": "Tap to flap!",
    "icon": "bird",
    "color": [255, 200, 50]
  }
]
```

## Usage

```bash
sudo python3 -u launcher.py
```

### Button Controls

| Context    | Action                      | Effect                   |
| ---------- | --------------------------- | ------------------------ |
| **Menu**   | Short press                 | Cycle to next app        |
| **Menu**   | Long press (hold > 0.5s)    | Launch selected app      |
| **In app** | Triple-press (3 quick taps) | Exit app, return to menu |

## Hardware Driver (`whisplay_hw.py`)

The launcher ships a shared Python driver at `settings/whisplay_hw.py` that handles the ST7789 display (SPI), backlight PWM, RGB LED, and button on the Whisplay HAT. It replaces the PiSugar `WhisPlay.py` driver, which doesn't work on Bookworm-based Pi OS (missing `GPIO.setmode()` for the lgpio backend).

When the launcher spawns an app, it automatically adds `settings/` to `PYTHONPATH`, so any app can import the driver directly:

```python
from whisplay_hw import WhisPlayBoard

board = WhisPlayBoard(backlight=80)

# Display a Pillow RGB image (240x280) — handles RGB565 conversion internally
board.display_image(img)

# Backlight (0-100), RGB LED (0-255 per channel)
board.set_backlight(70)
board.set_rgb(255, 0, 0)

# Button callbacks
board.on_button_press(my_press_fn)
board.on_button_release(my_release_fn)

# Cleanup on exit
board.cleanup()
```

Display dimensions are available as module-level constants:

```python
from whisplay_hw import LCD_W, LCD_H  # 240, 280
```

> **Performance note:** `display_image()` uses numpy for fast RGB565 conversion (~2ms per frame). Without numpy it falls back to a pure-Python loop (~150-300ms). numpy is pre-installed on Raspberry Pi OS, but if you're using a venv make sure to install it: `pip install numpy`

## Adding a New App

1. Clone or create your app in a sibling directory.

2. Add an entry to `apps.json`:

```json
{
  "name": "My App",
  "script": "../My-App/main.py",
  "description": "Short description",
  "icon": "game",
  "color": [100, 150, 255]
}
```

3. **Use the shared driver** — no need for the PiSugar `WhisPlay.py`:

```python
from whisplay_hw import WhisPlayBoard
from exit_helper import TriplePressExit

board = WhisPlayBoard()

def shutdown():
    board.cleanup()
    sys.exit(0)

TriplePressExit(board, on_press=my_press_fn, shutdown_fn=shutdown)
```

4. **Triple-press exit** lets users return to the launcher. The `TriplePressExit` helper wraps your button callbacks and detects three rapid presses within 1.5 seconds. You can also implement it manually:

```python
_triple_press_times = []
TRIPLE_WINDOW_S = 1.5

def on_button_press():
    now = time.time()
    _triple_press_times.append(now)
    _triple_press_times[:] = [
        t for t in _triple_press_times if now - t <= TRIPLE_WINDOW_S
    ]
    if len(_triple_press_times) >= 3:
        _triple_press_times.clear()
        shutdown()  # your cleanup function
        return
    # ... rest of your button logic ...
```

### Available Icon Types

| Icon value   | Draws                |
| ------------ | -------------------- |
| `music`      | Pair of eighth notes |
| `bird`       | Bird silhouette      |
| `game`       | Gamepad              |
| `settings`   | Gear                 |
| _(anything)_ | Dot grid (fallback)  |

## Run on Boot (systemd)

```bash
sudo nano /etc/systemd/system/whisplay-launcher.service
```

```ini
[Unit]
Description=Whisplay Launcher
After=network-online.target
Wants=network-online.target

[Service]
ExecStartPre=/bin/sleep 5
ExecStart=/usr/bin/python3 -u /home/<user>/Whisplay/example/Whisplay-Launcher/launcher.py
WorkingDirectory=/home/<user>/Whisplay/example/Whisplay-Launcher
User=root
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable whisplay-launcher
sudo systemctl start whisplay-launcher
```

If you previously had `whisplay-spotify.service`, disable it first:

```bash
sudo systemctl disable whisplay-spotify
```

## License

MIT
