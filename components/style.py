from typing import Literal
import pywinstyles
import sys
from tkinter import ttk

FAMILY = "Segoe UI"

type Theme = Literal["light", "dark"]
type ThemeCapital = Literal["Dark", "Light"]


def lower_theme(theme: ThemeCapital) -> Theme:
    """Convert ThemeCapital to Theme"""
    theme_map: dict[ThemeCapital, Theme] = {
        "Dark": "dark",
        "Light": "light",
    }
    return theme_map.get(theme, "light")


def configure_styles():
    """Configure custom ttk styles with fonts"""
    style = ttk.Style()

    # Configure custom label style
    style.configure("Custom.TLabel", font=(FAMILY, 12))

    # Configure custom labelframe style
    style.configure("Custom.TLabelFrame.Label", font=(FAMILY, 14, "bold"))

    # Configure accent label style
    style.configure("Accent.TLabel", font=(FAMILY, 12, "bold"), foreground="#0066CC")

    return style


def apply_theme_to_titlebar(root, theme: Theme):
    """Apply theme to the title bar on Windows

    code from: https://github.com/rdbende/Sun-Valley-ttk-theme?tab=readme-ov-file
    """
    version = sys.getwindowsversion()

    if version.major == 10 and version.build >= 22000:
        # Set the title bar color to the background color on Windows 11 for better appearance
        pywinstyles.change_header_color(root, "#1c1c1c" if theme == "dark" else "#fafafa")
    elif version.major == 10:
        # NOTE: version.major == 10 means Windows 10 or 11
        pywinstyles.apply_style(root, "dark" if theme == "dark" else "normal")

        # A hacky way to update the title bar's color on Windows 10 (it doesn't update instantly like on Windows 11)
        root.wm_attributes("-alpha", 0.99)
        root.wm_attributes("-alpha", 1)
