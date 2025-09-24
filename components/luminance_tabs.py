import tkinter as tk
from tkinter import ttk
from components.luminance_controller import LuminanceControllerFrame
from components.all_luminance_controller import AllLuminanceControllerFrame


class LuminanceTabsFrame(ttk.Frame):
    def __init__(self, parent, luminance: tk.IntVar, luminances: list[tk.IntVar], root):
        super().__init__(parent)
        self.luminances = luminances
        self.root = root

        # Create notebook for tabs
        self.notebook = ttk.Notebook(self)
        self.notebook.pack(fill=tk.BOTH, expand=True)

        # Create individual monitor control tab
        self.individual_frame = LuminanceControllerFrame(self.notebook, luminances)

        # Create all monitors control tab
        self.all_frame = AllLuminanceControllerFrame(self.notebook, luminance, luminances, root, self.individual_frame)

        self.notebook.add(self.all_frame, text="Control All Monitors")
        self.notebook.add(self.individual_frame, text="Control Individual Monitor")

        # Store reference for external updates
        self.current_luminance_frame = self.individual_frame
