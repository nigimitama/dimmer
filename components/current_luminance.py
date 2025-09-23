import tkinter as tk
from tkinter import ttk
from components.constants import FONT_SIZE_H2, FONT_SIZE


class CurrentLuminanceFrame(ttk.Frame):
    def __init__(self, parent, luminances: list[tk.StringVar]):
        super().__init__(parent)
        self.luminances = luminances

        # Title label
        title_label = ttk.Label(self, text="Current Luminance", font=("TkDefaultFont", FONT_SIZE_H2, "bold"))
        title_label.pack(anchor="w", pady=(0, 5))

        # Create monitor labels
        for i, luminance in enumerate(luminances):
            monitor_frame = ttk.Frame(self)
            monitor_frame.pack(fill=tk.X, pady=2)

            monitor_label = ttk.Label(monitor_frame, text=f"Monitor {i + 1}:", font=("TkDefaultFont", FONT_SIZE))
            monitor_label.pack(side=tk.LEFT)

            luminance_label = ttk.Label(monitor_frame, textvariable=luminance, font=("TkDefaultFont", FONT_SIZE))
            luminance_label.pack(side=tk.LEFT, padx=(10, 0))
