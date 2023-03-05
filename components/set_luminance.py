import flet as ft
from components.constants import FONT_SIZE_H2, FONT_SIZE
from modules import monitor


def set_luminance(page: ft.Page, luminances: list[ft.Text]) -> ft.Row:
    input_height = int(FONT_SIZE * 2)

    input = ft.TextField(
        value="50",
        text_align=ft.TextAlign.RIGHT,
        width=50,
        height=input_height,
        text_size=FONT_SIZE
    )

    def _set_and_update(e):
        # set
        monitor.set_luminance(int(input.value))

        # update
        values = monitor.get_luminances()
        for luminance, value in zip(luminances, values):
            luminance.value = value
        page.update()


    row = ft.Row(
        [
            input,
            ft.ElevatedButton("Set", on_click=_set_and_update, height=input_height),
        ],
        alignment=ft.MainAxisAlignment.START
    )

    col = ft.Column(
        [
            ft.Text("Set Luminance", size=FONT_SIZE_H2),
            row,
        ],
        horizontal_alignment=ft.CrossAxisAlignment.START
    )

    return ft.Container(
        col,
        alignment=ft.alignment.center_left,
        margin=ft.Margin(10, 10, 10, 10)
    )
