import schedule

from modules import monitor


def update_display():
    """Update the display with the latest information"""
    schedule.every().minute.do(update_luminance_vars)


def update_luminance_vars(luminance_vars):
    """Fetch and update luminance variables"""
    values = monitor.get_luminances()
    for luminance_var, monitor_value in zip(luminance_vars, values):
        luminance_var.set(str(monitor_value))
