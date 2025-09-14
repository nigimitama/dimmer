import json
from typing import Any
from pathlib import Path


class Storage:
    """Save data into json"""

    def __init__(self, path=Path("dimmer.json")) -> None:
        self.path = path
        self.data = self.load() or {}

    def load(self):
        if not self.path.exists():
            return None

        with open(self.path, "r") as f:
            data = json.load(f)
        return data

    def save(self):
        with open(self.path, "w") as f:
            json.dump(self.data, f)

    def get(self, key: str):
        return self.data.get(key)

    def set(self, key: str, value: Any):
        self.data[key] = value
        self.save()
