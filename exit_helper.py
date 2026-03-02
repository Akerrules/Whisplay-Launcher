"""
Triple-press exit handler for Whisplay apps.

Drop-in helper that detects three rapid button presses and exits
the app back to the Whisplay Launcher.  Works alongside each app's
own short-press / long-press logic by forwarding normal events to
the app's original callbacks.

Usage (inside any Whisplay app):
    from exit_helper import TriplePressExit

    # after board and button callbacks are set up:
    triple = TriplePressExit(
        board,
        on_press=my_on_button_press,
        on_release=my_on_button_release,
        shutdown_fn=my_shutdown,
    )
"""

import time
import sys

TRIPLE_WINDOW_S = 1.5
REQUIRED_PRESSES = 3


class TriplePressExit:
    """Wraps WhisPlayBoard button callbacks with a triple-press exit detector."""

    def __init__(self, board, on_press=None, on_release=None,
                 shutdown_fn=None, window=TRIPLE_WINDOW_S):
        self._board = board
        self._user_on_press = on_press
        self._user_on_release = on_release
        self._shutdown_fn = shutdown_fn
        self._window = window
        self._press_times: list[float] = []

        board.on_button_press(self._handle_press)
        board.on_button_release(self._handle_release)

    def _handle_press(self):
        now = time.time()
        self._press_times.append(now)
        # keep only presses inside the detection window
        self._press_times = [
            t for t in self._press_times if now - t <= self._window
        ]
        if len(self._press_times) >= REQUIRED_PRESSES:
            self._press_times.clear()
            self._exit_to_launcher()
            return

        if self._user_on_press:
            self._user_on_press()

    def _handle_release(self):
        if self._user_on_release:
            self._user_on_release()

    def _exit_to_launcher(self):
        if self._shutdown_fn:
            self._shutdown_fn()
        else:
            sys.exit(0)
