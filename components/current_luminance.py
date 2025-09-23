import tkinter as tk
from tkinter import ttk


class CurrentLuminanceFrame(ttk.LabelFrame):
    def __init__(self, parent, luminances: list[tk.StringVar]):
        super().__init__(parent, text="Current Luminance", padding=15)
        self.luminances = luminances

        # Create monitor labels with improved styling
        for i, luminance in enumerate(luminances):
            monitor_frame = ttk.Frame(self)
            monitor_frame.pack(fill=tk.X, pady=8)

            # Monitor label
            monitor_label = ttk.Label(monitor_frame, text=f"Monitor {i + 1}:", style="Custom.TLabel")
            monitor_label.pack(side=tk.LEFT)

            # Value label with accent styling
            value_frame = ttk.Frame(monitor_frame)
            value_frame.pack(side=tk.LEFT, padx=(15, 0))

            luminance_label = ttk.Label(value_frame, textvariable=luminance, style="Accent.TLabel")
            luminance_label.pack(side=tk.LEFT)

            ttk.Label(value_frame, text="%", style="Custom.TLabel").pack(side=tk.LEFT)
