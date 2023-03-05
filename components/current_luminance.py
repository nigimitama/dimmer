import flet as ft
from components.constants import FONT_SIZE_H2, FONT_SIZE
from modules import monitor


def current_luminance(luminances: list[ft.Text]) -> ft.Row:
    rows = []
    for i, luminance in enumerate(luminances):
        rows.append(ft.Row([ft.Text(f"Monitor {i + 1}:", size=FONT_SIZE), luminance]))

    return ft.Container(
        ft.Column([
            ft.Text("Current Luminance", size=FONT_SIZE_H2),
            *rows,
        ]),
        alignment=ft.alignment.center_left,
        margin=ft.Margin(10, 10, 10, 10)
    )
