import tkinter as tk
from tkinter import ttk
from modules import monitor
from components.custom_widgets import LuminanceScale


class LuminanceControllerFrame(ttk.Frame):
    def __init__(self, parent, luminances: list[tk.StringVar]):
        super().__init__(parent, padding=15)
        self.luminances = luminances
        self.sliders = []
        self.value_labels = []

        # Create monitor controls with sliders
        for i, luminance in enumerate(luminances):
            # Main container for each monitor
            monitor_container = ttk.Frame(self)
            monitor_container.pack(fill=tk.X, pady=10)

            # Top row: Monitor label and value
            top_frame = ttk.Frame(monitor_container)
            top_frame.pack(fill=tk.X, pady=(0, 5))

            # Monitor label
            monitor_label = ttk.Label(top_frame, text=f"Monitor {i + 1}:", style="Custom.TLabel")
            monitor_label.pack(side=tk.LEFT)

            # Value display
            value_frame = ttk.Frame(top_frame)
            value_frame.pack(side=tk.RIGHT)

            luminance_label = ttk.Label(value_frame, textvariable=luminance, style="Accent.TLabel")
            luminance_label.pack(side=tk.LEFT)
            self.value_labels.append(luminance_label)

            ttk.Label(value_frame, text="%", style="Custom.TLabel").pack(side=tk.LEFT)

            # Bottom row: Slider
            slider_frame = ttk.Frame(monitor_container)
            slider_frame.pack(fill=tk.X)

            # Get initial value for slider
            try:
                initial_value = int(luminance.get())
            except (ValueError, TypeError):
                initial_value = 50

            # Create slider using LuminanceScale
            slider = LuminanceScale(
                slider_frame,
                command=lambda val, idx=i: self._on_slider_change(val, idx),
            )
            slider.set(initial_value)
            slider.pack(fill=tk.X)
            self.sliders.append(slider)

    def _on_slider_change(self, value, monitor_index):
        """Handle slider value change for individual monitor"""
        try:
            new_value = int(float(value))

            # Set luminance for the specific monitor
            monitor.set_monitor_luminance(monitor_index, new_value)

            # Update the corresponding luminance variable
            if monitor_index < len(self.luminances):
                self.luminances[monitor_index].set(str(new_value))

        except (ValueError, TypeError):
            pass

    def update_from_external(self):
        """Update sliders when luminance is changed externally"""
        try:
            current_values = monitor.get_luminances()
            for i, slider in enumerate(self.sliders):
                if i < len(current_values):
                    value = current_values[i]
                    slider.set(value)
                    self.luminances[i].set(str(value))
        except Exception:
            pass
