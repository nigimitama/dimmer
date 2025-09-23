"""System tray icon management for Dimmer application"""

import threading
import tkinter as tk
from PIL import Image, ImageDraw
import pystray
from pystray import MenuItem as item


def create_icon() -> Image.Image:
    """Create a simple icon for the system tray"""
    # Create a 64x64 image with transparent background
    image = Image.new("RGBA", (64, 64), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    # Draw a simple brightness/monitor icon
    # Monitor frame
    draw.rectangle([8, 16, 56, 40], outline=(255, 255, 255, 255), width=2)
    # Monitor stand
    draw.rectangle([28, 40, 36, 48], fill=(255, 255, 255, 255))
    draw.rectangle([20, 48, 44, 52], fill=(255, 255, 255, 255))

    # Brightness rays around the monitor
    for i in range(0, 360, 45):
        import math

        angle = math.radians(i)
        x1 = 32 + 24 * math.cos(angle)
        y1 = 28 + 12 * math.sin(angle)
        x2 = 32 + 30 * math.cos(angle)
        y2 = 28 + 15 * math.sin(angle)
        draw.line([x1, y1, x2, y2], fill=(255, 255, 255, 255), width=2)

    return image


class SystemTrayManager:
    """Manages the system tray icon and menu"""

    def __init__(self, root: tk.Tk):
        self.root = root
        self.icon = None
        self.is_visible = True

    def create_tray_icon(self):
        """Create and configure the system tray icon"""
        icon_image = create_icon()

        menu = pystray.Menu(item("Show/Hide", self.toggle_window, default=True), item("Exit", self.quit_application))

        self.icon = pystray.Icon("Dimmer", icon_image, "Dimmer - Monitor Brightness Control", menu)

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
