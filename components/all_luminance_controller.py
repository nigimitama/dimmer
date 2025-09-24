import tkinter as tk
from tkinter import ttk
from modules import monitor
from components.custom_widgets import LuminanceScale
from modules.performance import measure_time


class AllLuminanceControllerFrame(ttk.Frame):
    def __init__(self, parent, luminances: list[tk.StringVar], root, current_luminance_frame=None):
        super().__init__(parent, padding=15)
        self.luminances = luminances
        self.root = root
        self.current_luminance_frame = current_luminance_frame

        # Control frame
        control_frame = ttk.Frame(self)
        control_frame.pack(fill=tk.X)
        ttk.Label(control_frame, text="Luminance:", style="Custom.TLabel").pack(side=tk.LEFT, padx=(0, 20))

        # Get current luminance for initial value
        current_luminances = monitor.get_luminances()
        initial_value = current_luminances[0] if current_luminances else 50

        # Scale for luminance control with modern styling
        self.luminance_var = tk.IntVar(value=initial_value)
        self.scale = LuminanceScale(control_frame, variable=self.luminance_var, command=self._set_and_update)
        self.scale.pack(side=tk.LEFT, padx=(0, 20))

        # Current value display
        value_frame = ttk.Frame(control_frame)
        value_frame.pack(side=tk.LEFT)

        self.current_value_label = ttk.Label(value_frame, text=str(initial_value), style="Accent.TLabel")
        self.current_value_label.pack(side=tk.LEFT)

        ttk.Label(value_frame, text="%", style="Custom.TLabel").pack(side=tk.LEFT)

    @measure_time("set_and_update")
    def _set_and_update(self, value: str):
        """Update luminance when scale changes"""
        try:
            print("_set_and_update called with value:", value)
            new_luminance = int(float(value))  # CustomScale already handles conversion

            # Update the current value label
            self.current_value_label.config(text=str(new_luminance))

            # Set luminance
            monitor.set_luminance(new_luminance)

            # Update luminance variables
            values = monitor.get_luminances()
            for luminance_var, monitor_value in zip(self.luminances, values):
                luminance_var.set(str(monitor_value))

            # Update individual monitor sliders if available
            if self.current_luminance_frame and hasattr(self.current_luminance_frame, "update_from_external"):
                self.current_luminance_frame.update_from_external()

        except (ValueError, TypeError):
            pass
