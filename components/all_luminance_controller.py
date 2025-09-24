import tkinter as tk
from tkinter import ttk
from modules import monitor
from components.custom_widgets import LuminanceScale
from modules.performance import measure_time


class AllLuminanceControllerFrame(ttk.Frame):
    def __init__(self, parent, luminance: tk.IntVar, luminances: list[tk.IntVar], root):
        super().__init__(parent, padding=15)
        self.luminance = luminance
        self.luminances = luminances
        self.root = root

        # Control frame
        control_frame = ttk.Frame(self)
        control_frame.pack(fill=tk.X)
        ttk.Label(control_frame, text="Luminance:", style="Custom.TLabel").pack(side=tk.LEFT, padx=(0, 20))

        # Scale for luminance control with modern styling
        self.scale = LuminanceScale(control_frame, variable=self.luminance, command=self._set_and_update)
        self.scale.pack(side=tk.LEFT, padx=(0, 20))

        # Current value display
        value_frame = ttk.Frame(control_frame)
        value_frame.pack(side=tk.LEFT)

        self.current_value_label = ttk.Label(value_frame, textvariable=self.luminance, style="Accent.TLabel")
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
                luminance_var.set(monitor_value)

        except (ValueError, TypeError):
            pass
