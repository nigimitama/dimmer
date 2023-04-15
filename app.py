import time
import schedule
import flet as ft
from components.current_luminance import current_luminance
from components.set_luminance import set_luminance
from modules import monitor


def setup_luminance_values() -> list[ft.Text]:
    values = monitor.get_luminances()
    return [ft.Text(value=str(value)) for value in values]


def main(page: ft.Page):
    page.title = "Dimmer"
    page.window_width = 300
    page.window_height = 400
    page.vertical_alignment = ft.MainAxisAlignment.START

    luminances = setup_luminance_values()
    page.add(
        current_luminance(luminances),
        set_luminance(page, luminances)
    )

    def _set_and_update(new_luminance: int):
        # set
        monitor.set_luminance(new_luminance)

        # update current luminance displaying
        values = monitor.get_luminances()
        for luminance, value in zip(luminances, values):
            luminance.value = value
        page.update()


    schedule.every().day.at("06:00").do(lambda :_set_and_update(new_luminance=60))
    schedule.every().day.at("07:00").do(lambda :_set_and_update(new_luminance=70))
    schedule.every().day.at("08:00").do(lambda :_set_and_update(new_luminance=80))
    schedule.every().day.at("19:00").do(lambda :_set_and_update(new_luminance=50))
    schedule.every().day.at("20:00").do(lambda :_set_and_update(new_luminance=40))
    schedule.every().day.at("21:00").do(lambda :_set_and_update(new_luminance=30))
    schedule.every().day.at("22:00").do(lambda :_set_and_update(new_luminance=20))
    schedule.every().day.at("23:00").do(lambda :_set_and_update(new_luminance=10))

    while True:
        schedule.run_pending()
        time.sleep(1)


if __name__ == '__main__':
    ft.app(target=main)
