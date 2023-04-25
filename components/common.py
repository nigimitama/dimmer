import flet as ft


def time_dropdown():
    hour = ft.Dropdown(
        label="Hour",
        options=[ft.dropdown.Option(f"{hour:0>2}") for hour in range(24)],
        width=65,
        value=f"{0:0>2}"
    )
    minute = ft.Dropdown(
        label="Minute",
        options=[ft.dropdown.Option(f"{minute:0>2}") for minute in range(60)],
        width=65,
        value=f"{0:0>2}"
    )
    return ft.Container(
        ft.Row([ hour, minute ]),
        alignment=ft.alignment.center_left,
        margin=ft.Margin(10, 10, 10, 10)
    )


class TimeDropdown(ft.UserControl):
    def __init__(self, hour=0, minute=0, on_change=None):
        super().__init__()
        self.hour = ft.Dropdown(
            label="Hour",
            options=[ft.dropdown.Option(f"{hour:0>2}") for hour in range(24)],
            width=65,
            value=f"{hour:0>2}",
            on_change=on_change,
        )
        self.minute = ft.Dropdown(
            label="Minute",
            options=[ft.dropdown.Option(f"{minute:0>2}") for minute in range(60)],
            width=65,
            value=f"{minute:0>2}",
            on_change=on_change,
        )

    def build(self):
        return ft.Container(
            ft.Row([ self.hour, self.minute ]),
            alignment=ft.alignment.center_left,
            margin=ft.Margin(10, 10, 10, 10)
        )
