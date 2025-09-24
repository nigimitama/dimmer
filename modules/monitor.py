from monitorcontrol import get_monitors


def get_monitors_list():
    """Get list of available monitors"""
    return list(get_monitors())


def get_luminances() -> list[int]:
    values = []
    for monitor in get_monitors():
        with monitor:
            values.append(monitor.get_luminance())
    return values


def get_monitor_luminance(monitor_index: int) -> int:
    """Get luminance for a specific monitor"""
    monitors = get_monitors_list()
    if 0 <= monitor_index < len(monitors):
        monitor = monitors[monitor_index]
        with monitor:
            return monitor.get_luminance()
    return 0


def set_luminance(value: int):
    """Set luminance for all monitors"""
    for monitor in get_monitors():
        with monitor:
            try:
                monitor.set_luminance(value)
            except Exception:
                pass


def set_monitor_luminance(monitor_index: int, value: int):
    """Set luminance for a specific monitor"""
    monitors = get_monitors_list()
    if 0 <= monitor_index < len(monitors):
        monitor = monitors[monitor_index]
        with monitor:
            try:
                monitor.set_luminance(value)
            except Exception:
                pass
