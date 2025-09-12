import time
import schedule
import flet as ft
from components.current_luminance import current_luminance
from components.set_luminance import set_luminance
from components.schedule import ScheduleControl
from modules import monitor


def setup_luminance_vars() -> list[ft.Text]:
    values = monitor.get_luminances()
    return [ft.Text(value=str(value)) for value in values]


def main(page: ft.Page):
    page.title = "Dimmer"
    page.window_width = 600
    page.window_height = 600
    page.scroll = ft.ScrollMode("adaptive")
    page.vertical_alignment = ft.MainAxisAlignment.START

    luminance_vars = setup_luminance_vars()
    page.add(
        current_luminance(luminance_vars),
        set_luminance(page, luminance_vars),
        ScheduleControl(page, luminance_vars)
    )

    while True:
        schedule.run_pending()
        time.sleep(1)


if __name__ == '__main__':
    ft.app(target=main)
