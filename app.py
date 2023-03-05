import flet as ft
from components.current_luminance import current_luminance
from components.set_luminance import set_luminance
from modules import monitor


def get_luminance_values() -> list[ft.Text]:
    values = monitor.get_luminances()
    return [ft.Text(value=str(value)) for value in values]


def main(page: ft.Page):
    page.title = "Dimmer"
    page.window_width = 300
    page.window_height = 400
    page.vertical_alignment = ft.MainAxisAlignment.START

    luminances = get_luminance_values()
    page.add(
        current_luminance(luminances),
        set_luminance(page, luminances)
    )



if __name__ == '__main__':
    ft.app(target=main)
