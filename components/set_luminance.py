import flet as ft
from components.constants import FONT_SIZE_H2, FONT_SIZE
from modules import monitor


def set_luminance(page: ft.Page, luminances: list[ft.Text]) -> ft.Container:

    def _set_and_update(e: ft.ControlEvent):
        if not isinstance(e.control.value, (int, float)):
            return

        new_luminance = int(e.control.value)

        # set luminance
        monitor.set_luminance(new_luminance)

        # update current luminance
        current_luminance.value = str(new_luminance)
        values = monitor.get_luminances()
        for luminance, value in zip(luminances, values):
            luminance.value = str(value)
        page.update()

    input = ft.Slider(
        value=50, min=0, max=100,
        divisions=10, label="{value}",
        width=150,
        on_change=_set_and_update
    )

    current_luminance = ft.Text(str(input.value))

    row = ft.Row(
        [
            input,
            current_luminance
        ],
        alignment=ft.MainAxisAlignment.START
    )

    col = ft.Column(
        [
            ft.Text("Set Luminance", size=FONT_SIZE_H2),
            ft.Text("set luminance for all monitors immediately", size=FONT_SIZE),
            row,
        ],
        horizontal_alignment=ft.CrossAxisAlignment.START
    )

    return ft.Container(
        col,
        alignment=ft.Alignment(-1, 0),
        margin=ft.margin.all(10)
    )
