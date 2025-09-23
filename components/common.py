"""Common utilities for tkinter components"""

from tkinter import ttk


def create_labeled_frame(parent, title: str, font_size: int = 12) -> ttk.LabelFrame:
    """Create a labeled frame with consistent styling"""
    frame = ttk.LabelFrame(parent, text=title)
    frame.configure(padding=(10, 10))
    return frame


def create_title_label(parent, text: str, font_size: int = 24) -> ttk.Label:
    """Create a title label with consistent styling"""
    return ttk.Label(parent, text=text, font=("TkDefaultFont", font_size, "bold"))


def create_description_label(parent, text: str, font_size: int = 12) -> ttk.Label:
    """Create a description label with consistent styling"""
    return ttk.Label(parent, text=text, font=("TkDefaultFont", font_size))


def bind_mousewheel(canvas, scrollbar):
    """Bind mousewheel scrolling to a canvas with scrollbar"""

    def _on_mousewheel(event):
        canvas.yview_scroll(int(-1 * (event.delta / 120)), "units")

    def _bind_to_mousewheel(event):
        canvas.bind_all("<MouseWheel>", _on_mousewheel)

    def _unbind_from_mousewheel(event):
        canvas.unbind_all("<MouseWheel>")

    canvas.bind("<Enter>", _bind_to_mousewheel)
    canvas.bind("<Leave>", _unbind_from_mousewheel)
