import time
import schedule
import threading
import flet as ft
from components.current_luminance import current_luminance
from components.set_luminance import set_luminance
from components.schedule import ScheduleControl
from modules import monitor


def setup_luminance_vars() -> list[ft.Text]:
    values = monitor.get_luminances()
    return [ft.Text(value=str(value)) for value in values]


def schedule_worker():
    """Run the schedule in a separate thread"""
    while True:
        schedule.run_pending()
        time.sleep(1)


def main(page: ft.Page):
    page.title = "Dimmer"
    page.window.width = 600
    page.window.height = 600
    page.scroll = ft.ScrollMode.ADAPTIVE
    page.vertical_alignment = ft.MainAxisAlignment.START

    luminance_vars = setup_luminance_vars()
    page.add(
        current_luminance(luminance_vars),
        set_luminance(page, luminance_vars),
        ScheduleControl(page, luminance_vars)
    )

    # Start the schedule worker in a separate thread
    schedule_thread = threading.Thread(target=schedule_worker, daemon=True)
    schedule_thread.start()


if __name__ == '__main__':
    ft.app(target=main)
