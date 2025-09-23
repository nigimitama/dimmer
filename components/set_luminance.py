import tkinter as tk
from tkinter import ttk
from components.constants import FONT_SIZE_H2, FONT_SIZE
from modules import monitor


class SetLuminanceFrame(ttk.Frame):
    def __init__(self, parent, luminances: list[tk.StringVar], root):
        super().__init__(parent)
        self.luminances = luminances
        self.root = root

        # Title label
        title_label = ttk.Label(self, text="Set Luminance", font=("TkDefaultFont", FONT_SIZE_H2, "bold"))
        title_label.pack(anchor="w", pady=(0, 5))

        # Description label
        desc_label = ttk.Label(
            self, text="set luminance for all monitors immediately", font=("TkDefaultFont", FONT_SIZE)
        )
        desc_label.pack(anchor="w", pady=(0, 10))

        # Control frame
        control_frame = ttk.Frame(self)
        control_frame.pack(fill=tk.X)

        # Get current luminance for initial value
        current_luminances = monitor.get_luminances()
        initial_value_percent = current_luminances[0] if current_luminances else 50
        initial_value = int(initial_value_percent / 10)  # [0-10] scale

        # Scale for luminance control
        self.luminance_var = tk.IntVar(value=initial_value)
        self.scale = ttk.Scale(
            control_frame,
            from_=0,
            to=10,
            orient=tk.HORIZONTAL,
            variable=self.luminance_var,
            command=self._set_and_update,
            length=200,
        )
        self.scale.pack(side=tk.LEFT, padx=(0, 10))

        # Current value label
        self.current_value_label = ttk.Label(
            control_frame, text=str(initial_value * 10), font=("TkDefaultFont", FONT_SIZE)
        )
        self.current_value_label.pack(side=tk.LEFT)

    def _set_and_update(self, value: str):
        """Update luminance when scale changes"""
        try:
            new_luminance = int(float(value) * 10)  # Convert [0-10] scale to [0-100]

            # Update the current value label
            self.current_value_label.config(text=str(new_luminance))

            # Set luminance
            monitor.set_luminance(new_luminance)

            # Update luminance variables
            values = monitor.get_luminances()
            for luminance_var, monitor_value in zip(self.luminances, values):
                luminance_var.set(str(monitor_value))

        except (ValueError, TypeError):
            pass
