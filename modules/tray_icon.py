"""System tray icon management for Dimmer application"""

from PIL import Image
import threading
import tkinter as tk
import darkdetect
import pystray
from pystray import MenuItem as item

from modules.path import resource_path


def load_icon():
    try:
        theme = darkdetect.theme() or "light"
        path = resource_path(f"assets/icon-{theme}.ico")
        with open(path, "rb") as f:
            icon_image = Image.open(f)
        return icon_image
    except Exception:
        return Image.new("RGB", (64, 64), color=(0, 0, 0))


class SystemTrayManager:
    """Manages the system tray icon and menu"""

    def __init__(self, root: tk.Tk):
        self.root = root
        self.icon = None
        self.is_visible = True

    def create_tray_icon(self):
        """Create and configure the system tray icon"""
        self.icon = pystray.Icon(
            name="Dimmer",
            icon=load_icon(),
            title="Dimmer - Monitor Brightness Control",
            menu=pystray.Menu(item("Show/Hide", self.toggle_window, default=True), item("Exit", self.quit_application)),
        )

    def toggle_window(self, icon=None, item=None):
        """Toggle window visibility"""
        if self.is_visible:
            self.root.withdraw()
            self.is_visible = False
        else:
            self.root.deiconify()
            self.root.lift()
            self.root.focus_force()
            self.is_visible = True

    def hide_window(self):
        """Hide the main window"""
        self.root.withdraw()
        self.is_visible = False

    def quit_application(self, icon=None, item=None):
        """Quit the entire application"""
        if self.icon:
            self.icon.stop()
        self.root.quit()

    def run_tray(self):
        """Run the system tray icon in a separate thread"""
        if self.icon:
            self.icon.run()

    def start_tray(self):
        """Start the system tray in a separate thread"""
        self.create_tray_icon()
        tray_thread = threading.Thread(target=self.run_tray, daemon=True)
        tray_thread.start()
