import time
import schedule
import threading
import tkinter as tk
from tkinter import ttk
from components.current_luminance import CurrentLuminanceFrame
from components.set_luminance import SetLuminanceFrame
from components.schedule import ScheduleFrame
from modules import monitor
from modules.tray_icon import SystemTrayManager


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
    root.geometry("600x600")

    # Setup system tray
    tray_manager = SystemTrayManager(root)

    # Override the close button to hide instead of exit
    def on_closing():
        tray_manager.hide_window()

    root.protocol("WM_DELETE_WINDOW", on_closing)

    # Setup shared luminance variables
    luminance_vars = setup_luminance_vars(root)

    # Create main frame with scrollable content
    main_frame = ttk.Frame(root)
    main_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)

    # Create components
    current_luminance_frame = CurrentLuminanceFrame(main_frame, luminance_vars)
    current_luminance_frame.pack(fill=tk.X, pady=(0, 10))

    set_luminance_frame = SetLuminanceFrame(main_frame, luminance_vars, root)
    set_luminance_frame.pack(fill=tk.X, pady=(0, 10))

    schedule_frame = ScheduleFrame(main_frame, luminance_vars, root)
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
