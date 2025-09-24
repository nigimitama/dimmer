import time
import schedule
import threading
import tkinter as tk
from tkinter import ttk
import sv_ttk
from components.luminance_controller import LuminanceControllerFrame
from components.set_luminance import SetLuminanceFrame
from components.schedule import ScheduleFrame
from components.style import Theme, apply_theme_to_titlebar, configure_styles
from modules import monitor
from modules.tray_icon import SystemTrayManager
import darkdetect


def setup_luminance_vars(root):
    """Setup luminance variables as StringVar objects"""
    values = monitor.get_luminances()
    return [tk.StringVar(root, str(value)) for value in values]


def schedule_worker():
    """Run the schedule in a separate thread"""
    while True:
        schedule.run_pending()
        time.sleep(1)


def main():
    root = tk.Tk()
    root.title("Dimmer")
    root.geometry("900x700")

    # set dark or light theme
    system_setting: Theme | None = darkdetect.theme()
    theme = system_setting or "light"
    sv_ttk.set_theme(theme)
    apply_theme_to_titlebar(root, theme)

    # Configure custom styles
    configure_styles()

    # Setup system tray
    tray_manager = SystemTrayManager(root)

    # Override the close button to hide instead of exit
    def on_closing():
        tray_manager.hide_window()

    root.protocol("WM_DELETE_WINDOW", on_closing)

    # Setup shared luminance variables
    luminance_vars = setup_luminance_vars(root)

    # Create main frame with modern styling
    main_frame = ttk.Frame(root)
    main_frame.pack(fill=tk.BOTH, expand=True, padx=20, pady=20)

    # Create components
    current_luminance_frame = LuminanceControllerFrame(main_frame, luminance_vars)
    current_luminance_frame.pack(fill=tk.X, pady=(0, 15))

    set_luminance_frame = SetLuminanceFrame(main_frame, luminance_vars, root, current_luminance_frame)
    set_luminance_frame.pack(fill=tk.X, pady=(0, 15))

    schedule_frame = ScheduleFrame(main_frame, luminance_vars, root, current_luminance_frame)
    schedule_frame.pack(fill=tk.BOTH, expand=True)

    # Start the schedule worker in a separate thread
    schedule_thread = threading.Thread(target=schedule_worker, daemon=True)
    schedule_thread.start()

    # Start system tray
    tray_manager.start_tray()

    # Start the main loop
    root.mainloop()


if __name__ == "__main__":
    main()
