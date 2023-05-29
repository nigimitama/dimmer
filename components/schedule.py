import flet as ft
from components.constants import FONT_SIZE_H2, FONT_SIZE
from modules.storage import Storage
from modules import monitor
import schedule


class ScheduleInput(ft.UserControl):
    """component for input hours, minutes and value of luminance"""

    def __init__(self, hour=0, minute=0, luminance=50, on_change=None):
        super().__init__()
        self.hour = ft.Dropdown(
            label="Hour",
            options=[ft.dropdown.Option(f"{hour:0>2}") for hour in range(24)],
            width=65,
            value=f"{hour:0>2}",
            on_change=on_change
        )
        self.minute = ft.Dropdown(
            label="Minute",
            options=[ft.dropdown.Option(f"{minute:0>2}") for minute in range(60)],
            width=65,
            value=f"{minute:0>2}",
            on_change=on_change
        )
        self.luminance = ft.TextField(label="Luminance", width=90, value=luminance, on_change=on_change)
        self.delete_button = ft.ElevatedButton("Delete", on_click=self._delete)
        self.on_change = on_change

    def build(self):
        return ft.Container(
            ft.Row([ self.hour, self.minute, self.luminance, self.delete_button ]),
            alignment=ft.alignment.center_left,
            margin=ft.Margin(10, 10, 10, 10)
        )

    def _delete(self, e):
        self.visible = False
        if self.on_change != None:
            self.on_change()
        self.update()


class ScheduleControl(ft.UserControl):

    def __init__(self, page: ft.Page, luminance_vars: list[ft.Text]) -> None:
        super().__init__()
        self.page = page
        self.luminance_vars = luminance_vars
        self.schedules = ft.Column()
        self.add_button = ft.ElevatedButton("Add", on_click=self._add_schedule)
        self.components = ft.Container(
            ft.Column([
                ft.Text("Schedule", size=FONT_SIZE_H2),
                ft.Text("luminance will be set at the scheduled time everyday", size=FONT_SIZE),
                self.schedules,
                self.add_button
            ]),
            alignment=ft.alignment.center_left,
            margin=ft.Margin(10, 10, 10, 50)
        )
        self.storage = Storage()

    def build(self):
        self._load()
        return self.components

    def _add_schedule(self, e = None):
        self.schedules.controls.append(ScheduleInput(on_change=self._save))
        self.update()

    def _load(self):
        values = self.storage.get(key="schedule") or []
        for (hour, minute, luminance) in values:
            # print(hour, minute, luminance)
            schedule_input = ScheduleInput(
                hour=hour,
                minute=minute,
                luminance=luminance,
                on_change=self._save
            )
            self.schedules.controls.append(schedule_input)
        self._update_jobs()

    def _save(self, e = None):
        """save settings and update view"""
        values = []
        for schedule_input in self.schedules.controls:
            if schedule_input.visible:
                hour = int(schedule_input.hour.value)
                minute = int(schedule_input.minute.value)
                luminance = int(schedule_input.luminance.value)
                values.append([hour, minute, luminance])

        self.storage.set(key="schedule", value=values)

        self.update()
        self._update_jobs()

        # show flash notice
        self.page.snack_bar = ft.SnackBar(ft.Text("Changes were saved"))
        self.page.snack_bar.open = True
        self.page.update()

    def _update_jobs(self):
        """update scheduled jobs by given scheudle inputs"""

        schedule.clear()
        for schedule_input in self.schedules.controls:
            if schedule_input.visible:
                hour: str = schedule_input.hour.value
                minute: str = schedule_input.minute.value
                luminance: int = int(schedule_input.luminance.value)

                job = ScheduledJob(page=self.page, luminance_vars=self.luminance_vars, new_luminance=luminance)
                schedule.every().day.at(f"{hour}:{minute}").do(job.set_and_update)


class ScheduledJob:

    def __init__(self, page: ft.Page, luminance_vars: list, new_luminance: int) -> None:
        self.page = page
        self.new_luminance = new_luminance
        self.luminance_vars = luminance_vars

    def set_and_update(self):
        # set
        monitor.set_luminance(self.new_luminance)
        monitor.set_luminance(self.new_luminance)  # try twice because it does not work sometimes 
        # update current luminance displaying
        values = monitor.get_luminances()
        for luminance, value in zip(self.luminance_vars, values):
            luminance.value = value
        self.page.update()
