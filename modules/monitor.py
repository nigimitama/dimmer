from monitorcontrol import get_monitors


def get_luminances() -> list[int]:
    values = []
    for monitor in get_monitors():
        with monitor:
            values.append(monitor.get_luminance())
    return values


def set_luminance(value: int):
    for monitor in get_monitors():
        with monitor:
            try:
                monitor.set_luminance(value)
            except Exception:
                pass
