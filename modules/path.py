import sys
import os


def resource_path(relative_path: str) -> str:
    if hasattr(sys, "_MEIPASS"):
        # production environment uses the temporary directory in sys._MEIPASS (if it is build with --one-file option)
        return os.path.join(sys._MEIPASS, relative_path)  # type: ignore

    # development environment uses the current directory
    return os.path.join(os.path.abspath("."), relative_path)
