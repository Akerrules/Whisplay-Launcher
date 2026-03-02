"""
Minimal Whisplay HAT hardware driver.

Handles the ST7789 display (SPI), backlight PWM, RGB LED, and button
on the PiSugar Whisplay HAT.  Designed as a reusable drop-in for any
Whisplay Python app.
"""

import time
import spidev
import RPi.GPIO as GPIO

try:
    import numpy as np
    _HAS_NUMPY = True
except ImportError:
    _HAS_NUMPY = False
    print("WARNING: numpy not found — display will be very slow (~5 FPS)."
          "  Install with:  pip install numpy  or  sudo apt install python3-numpy")

# BCM pin assignments
PIN_DC = 27
PIN_RST = 4
PIN_BL = 22
PIN_BTN = 17
PIN_LED_R = 25
PIN_LED_G = 24
PIN_LED_B = 23

LCD_W = 240
LCD_H = 280
Y_OFFSET = 20


class WhisPlayBoard:
    def __init__(self, backlight=70):
        GPIO.setwarnings(False)
        GPIO.setmode(GPIO.BCM)

        # SPI for display
        self.spi = spidev.SpiDev()
        self.spi.open(0, 0)
        self.spi.max_speed_hz = 100_000_000
        self.spi.mode = 0

        # Display control pins
        GPIO.setup(PIN_DC, GPIO.OUT)
        GPIO.setup(PIN_RST, GPIO.OUT)

        # Backlight (PWM 1kHz, inverted: 0% duty = full bright)
        GPIO.setup(PIN_BL, GPIO.OUT)
        self._bl_pwm = GPIO.PWM(PIN_BL, 1000)
        self._bl_val = backlight
        self._bl_pwm.start(100 - backlight)

        # RGB LED (PWM 100Hz, inverted)
        GPIO.setup(PIN_LED_R, GPIO.OUT)
        GPIO.setup(PIN_LED_G, GPIO.OUT)
        GPIO.setup(PIN_LED_B, GPIO.OUT)
        self._pwm_r = GPIO.PWM(PIN_LED_R, 100)
        self._pwm_g = GPIO.PWM(PIN_LED_G, 100)
        self._pwm_b = GPIO.PWM(PIN_LED_B, 100)
        self._pwm_r.start(100)
        self._pwm_g.start(100)
        self._pwm_b.start(100)

        # Button (active-high when pressed based on launcher Rust code)
        GPIO.setup(PIN_BTN, GPIO.IN, pull_up_down=GPIO.PUD_UP)
        self._press_cb = None
        self._release_cb = None
        GPIO.add_event_detect(
            PIN_BTN, GPIO.BOTH,
            callback=self._btn_isr, bouncetime=50,
        )

        self._init_display()

    # -- Display ---------------------------------------------------------------

    def _cmd(self, command, data=None):
        GPIO.output(PIN_DC, 0)
        self.spi.writebytes([command])
        if data:
            GPIO.output(PIN_DC, 1)
            self.spi.writebytes2(data)

    def _init_display(self):
        GPIO.output(PIN_RST, 1)
        time.sleep(0.1)
        GPIO.output(PIN_RST, 0)
        time.sleep(0.1)
        GPIO.output(PIN_RST, 1)
        time.sleep(0.12)

        self._cmd(0x11)
        time.sleep(0.12)
        self._cmd(0x36, [0xC0])
        self._cmd(0x3A, [0x05])
        self._cmd(0xB2, [0x0C, 0x0C, 0x00, 0x33, 0x33])
        self._cmd(0xB7, [0x35])
        self._cmd(0xBB, [0x32])
        self._cmd(0xC2, [0x01])
        self._cmd(0xC3, [0x15])
        self._cmd(0xC4, [0x20])
        self._cmd(0xC6, [0x0F])
        self._cmd(0xD0, [0xA4, 0xA1])
        self._cmd(0xE0, [
            0xD0, 0x08, 0x0E, 0x09, 0x09, 0x05,
            0x31, 0x33, 0x48, 0x17, 0x14, 0x15, 0x31, 0x34,
        ])
        self._cmd(0xE1, [
            0xD0, 0x08, 0x0E, 0x09, 0x09, 0x15,
            0x31, 0x33, 0x48, 0x17, 0x14, 0x15, 0x31, 0x34,
        ])
        self._cmd(0x21)
        self._cmd(0x29)

    def _set_window(self, x0, y0, x1, y1):
        y0 += Y_OFFSET
        y1 += Y_OFFSET
        self._cmd(0x2A, [x0 >> 8, x0 & 0xFF, x1 >> 8, x1 & 0xFF])
        self._cmd(0x2B, [y0 >> 8, y0 & 0xFF, y1 >> 8, y1 & 0xFF])
        self._cmd(0x2C)

    def display_image(self, img):
        """Send a Pillow RGB Image (240x280) to the display."""
        if img.size != (LCD_W, LCD_H):
            img = img.resize((LCD_W, LCD_H))

        if _HAS_NUMPY:
            a = np.asarray(img.convert("RGB"), dtype=np.uint16)
            packed = ((a[:, :, 0] & 0xF8) << 8) | ((a[:, :, 1] & 0xFC) << 3) | (a[:, :, 2] >> 3)
            buf = packed.astype(">u2").tobytes()
        else:
            pixels = img.tobytes()
            n = LCD_W * LCD_H
            buf = bytearray(n * 2)
            for i in range(n):
                off = i * 3
                r, g, b = pixels[off], pixels[off + 1], pixels[off + 2]
                val = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)
                buf[i * 2] = (val >> 8) & 0xFF
                buf[i * 2 + 1] = val & 0xFF

        self._set_window(0, 0, LCD_W - 1, LCD_H - 1)
        GPIO.output(PIN_DC, 1)
        for off in range(0, len(buf), 4096):
            self.spi.writebytes2(buf[off:off + 4096])

    def fill_black(self):
        self._set_window(0, 0, LCD_W - 1, LCD_H - 1)
        GPIO.output(PIN_DC, 1)
        chunk = bytes(4096)
        total = LCD_W * LCD_H * 2
        for off in range(0, total, 4096):
            self.spi.writebytes2(chunk[:min(4096, total - off)])

    # -- Backlight -------------------------------------------------------------

    def set_backlight(self, pct):
        """Set backlight brightness (0..100)."""
        pct = max(0, min(100, pct))
        self._bl_val = pct
        self._bl_pwm.ChangeDutyCycle(100 - pct)

    def get_backlight(self):
        return self._bl_val

    # -- RGB LED ---------------------------------------------------------------

    def set_rgb(self, r, g, b):
        self._pwm_r.ChangeDutyCycle(100 - (r * 100 / 255))
        self._pwm_g.ChangeDutyCycle(100 - (g * 100 / 255))
        self._pwm_b.ChangeDutyCycle(100 - (b * 100 / 255))

    # -- Button ----------------------------------------------------------------

    def _btn_isr(self, channel):
        level = GPIO.input(PIN_BTN)
        if level:
            if self._press_cb:
                self._press_cb()
        else:
            if self._release_cb:
                self._release_cb()

    def on_button_press(self, cb):
        self._press_cb = cb

    def on_button_release(self, cb):
        self._release_cb = cb

    def is_button_pressed(self):
        return GPIO.input(PIN_BTN) == 1

    # -- Cleanup ---------------------------------------------------------------

    def cleanup(self):
        self.fill_black()
        self._bl_pwm.ChangeDutyCycle(100)
        self._pwm_r.ChangeDutyCycle(100)
        self._pwm_g.ChangeDutyCycle(100)
        self._pwm_b.ChangeDutyCycle(100)
        self._bl_pwm.stop()
        self._pwm_r.stop()
        self._pwm_g.stop()
        self._pwm_b.stop()
        GPIO.cleanup()
