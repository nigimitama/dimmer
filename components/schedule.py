import tkinter as tk
from tkinter import ttk, messagebox
from components.constants import FONT_SIZE_H2, FONT_SIZE
from modules.storage import Storage
from modules import monitor
import schedule


class ScheduleInputFrame(ttk.Frame):
    """Frame for input hours, minutes and value of luminance"""

    def __init__(self, parent, hour=0, minute=0, luminance=50, on_change=None, on_delete=None):
        super().__init__(parent)
        self.on_change = on_change
        self.on_delete = on_delete

        # Hour combobox
        self.hour_var = tk.StringVar(value=f"{hour:02d}")
        hour_label = ttk.Label(self, text="Hour:")
        hour_label.grid(row=0, column=0, padx=(0, 5), sticky="w")
        self.hour_combo = ttk.Combobox(
            self, textvariable=self.hour_var, values=[f"{h:02d}" for h in range(24)], width=8, state="readonly"
        )
        self.hour_combo.grid(row=0, column=1, padx=(0, 10))
        self.hour_combo.bind("<<ComboboxSelected>>", self._on_change)

        # Minute combobox
        self.minute_var = tk.StringVar(value=f"{minute:02d}")
        minute_label = ttk.Label(self, text="Minute:")
        minute_label.grid(row=0, column=2, padx=(0, 5), sticky="w")
        self.minute_combo = ttk.Combobox(
            self, textvariable=self.minute_var, values=[f"{m:02d}" for m in range(60)], width=8, state="readonly"
        )
        self.minute_combo.grid(row=0, column=3, padx=(0, 10))
        self.minute_combo.bind("<<ComboboxSelected>>", self._on_change)

        # Luminance entry
        self.luminance_var = tk.StringVar(value=str(luminance))
        luminance_label = ttk.Label(self, text="Luminance:")
        luminance_label.grid(row=0, column=4, padx=(0, 5), sticky="w")
        self.luminance_entry = ttk.Entry(self, textvariable=self.luminance_var, width=10)
        self.luminance_entry.grid(row=0, column=5, padx=(0, 10))
        self.luminance_entry.bind("<KeyRelease>", self._on_change)

        # Delete button
        self.delete_button = ttk.Button(self, text="Delete", command=self._delete)
        self.delete_button.grid(row=0, column=6)

    def _on_change(self, event=None):
        if self.on_change:
            self.on_change()

    def _delete(self):
        if self.on_delete:
            self.on_delete(self)

    def get_values(self):
        """Get the hour, minute, and luminance values"""
        try:
            hour = int(self.hour_var.get())
            minute = int(self.minute_var.get())
            luminance = int(self.luminance_var.get())
            return hour, minute, luminance
        except ValueError:
            return None


class ScheduleFrame(ttk.Frame):
    def __init__(self, parent, luminance_vars: list[tk.StringVar], root):
        super().__init__(parent)
        self.luminance_vars = luminance_vars
        self.root = root
        self.storage = Storage()
        self.schedule_inputs = []

        # Title label
        title_label = ttk.Label(self, text="Schedule", font=("TkDefaultFont", FONT_SIZE_H2, "bold"))
        title_label.pack(anchor="w", pady=(0, 5))

        # Description label
        desc_label = ttk.Label(
            self, text="luminance will be set at the scheduled time everyday", font=("TkDefaultFont", FONT_SIZE)
        )
        desc_label.pack(anchor="w", pady=(0, 10))

        # Scrollable frame for schedule inputs
        self.canvas = tk.Canvas(self)
        self.scrollbar = ttk.Scrollbar(self, orient="vertical", command=self.canvas.yview)
        self.scrollable_frame = ttk.Frame(self.canvas)

        self.scrollable_frame.bind("<Configure>", lambda e: self.canvas.configure(scrollregion=self.canvas.bbox("all")))

        self.canvas.create_window((0, 0), window=self.scrollable_frame, anchor="nw")
        self.canvas.configure(yscrollcommand=self.scrollbar.set)

        self.canvas.pack(side="left", fill="both", expand=True)
        self.scrollbar.pack(side="right", fill="y")

        # Add button
        add_button_frame = ttk.Frame(self)
        add_button_frame.pack(fill=tk.X, pady=(10, 0))

        self.add_button = ttk.Button(add_button_frame, text="Add Schedule", command=self._add_schedule)
        self.add_button.pack(anchor="w")

        self._load()

    def _add_schedule(self):
        schedule_input = ScheduleInputFrame(
            self.scrollable_frame, hour=0, minute=0, luminance=50, on_change=self._save, on_delete=self._delete_schedule
        )
        schedule_input.pack(fill=tk.X, pady=2)
        self.schedule_inputs.append(schedule_input)
        self._save()

    def _delete_schedule(self, schedule_input):
        if schedule_input in self.schedule_inputs:
            self.schedule_inputs.remove(schedule_input)
            schedule_input.destroy()
            self._save()

    def _load(self):
        values = self.storage.get(key="schedule") or []
        for hour, minute, luminance in values:
            schedule_input = ScheduleInputFrame(
                self.scrollable_frame,
                hour=hour,
                minute=minute,
                luminance=luminance,
                on_change=self._save,
                on_delete=self._delete_schedule,
            )
            schedule_input.pack(fill=tk.X, pady=2)
            self.schedule_inputs.append(schedule_input)
        self._update_jobs()

    def _save(self):
        """Save settings and update view"""
        values = []
        for schedule_input in self.schedule_inputs:
            result = schedule_input.get_values()
            if result:
                values.append(list(result))

        self.storage.set(key="schedule", value=values)
        self._update_jobs()

        # Show saved message
        messagebox.showinfo("Saved", "Changes were saved")

    def _update_jobs(self):
        """Update scheduled jobs by given schedule inputs"""
        schedule.clear()
        for schedule_input in self.schedule_inputs:
            result = schedule_input.get_values()
            if result:
                hour, minute, luminance = result
                job = ScheduledJob(root=self.root, luminance_vars=self.luminance_vars, new_luminance=luminance)
                schedule.every().day.at(f"{hour:02d}:{minute:02d}").do(job.set_and_update)


class ScheduledJob:
    def __init__(self, root, luminance_vars: list[tk.StringVar], new_luminance: int):
        self.root = root
        self.new_luminance = new_luminance
        self.luminance_vars = luminance_vars

    def set_and_update(self):
        # Set luminance
        monitor.set_luminance(self.new_luminance)
        monitor.set_luminance(self.new_luminance)  # try twice because it does not work sometimes

        # Update current luminance displaying
        values = monitor.get_luminances()
        for luminance_var, value in zip(self.luminance_vars, values):
            luminance_var.set(str(value))
