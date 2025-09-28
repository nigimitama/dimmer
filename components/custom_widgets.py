"""Custom tkinter widgets for the Dimmer application"""

import tkinter as tk
from tkinter import ttk


class CustomScale(ttk.Scale):
    """
    Custom Scale widget that supports step size for easier value control.

    This widget internally uses a scaled range and converts values to/from
    the actual range based on the stepsize parameter.
    """

    def __init__(self, parent, stepsize=1, **kwargs):
        """
        Initialize CustomScale with stepsize support.

        Args:
            parent: Parent widget
            stepsize: Step size for the scale (e.g., 10 means each step is 10 units)
            **kwargs: Standard ttk.Scale arguments (from_, to, etc.)
        """
        self.stepsize = stepsize

        # Extract the actual range values
        self.actual_from = kwargs.get("from_", 0)
        self.actual_to = kwargs.get("to", 100)

        # Calculate internal scale range (number of steps)
        self.internal_from = 0
        self.internal_to = int((self.actual_to - self.actual_from) / stepsize)

        # Modify kwargs for internal scale
        modified_kwargs = kwargs.copy()
        modified_kwargs["from_"] = self.internal_from
        modified_kwargs["to"] = self.internal_to

        # Remove variable if provided, we'll handle it ourselves
        self.external_variable = modified_kwargs.pop("variable", None)
        self.external_command = modified_kwargs.pop("command", None)

        # Create internal variable
        self.internal_variable = tk.IntVar()
        modified_kwargs["variable"] = self.internal_variable
        modified_kwargs["command"] = self._on_internal_change

        # Initialize the parent Scale
        super().__init__(parent, **modified_kwargs)

        # Set initial value if external variable was provided
        if self.external_variable:
            initial_value = self.external_variable.get()
            self.set_actual_value(initial_value)
            # Trace external variable changes
            self.external_variable.trace_add("write", self._on_external_change)

    def _on_internal_change(self, value):
        """Handle internal scale change and convert to actual value"""
        try:
            internal_value = int(float(value))
            actual_value = self._internal_to_actual(internal_value)

            # Update external variable if provided
            if self.external_variable:
                self.external_variable.set(actual_value)

            # Call external command if provided
            if self.external_command:
                self.external_command(str(actual_value))

        except (ValueError, TypeError):
            pass

    def _on_external_change(self, *args):
        """Handle external variable change and convert to internal value"""
        try:
            if self.external_variable:
                actual_value = self.external_variable.get()
                internal_value = self._actual_to_internal(actual_value)
                self.internal_variable.set(internal_value)
        except (ValueError, TypeError):
            pass

    def _actual_to_internal(self, actual_value):
        """Convert actual value to internal scale value"""
        try:
            actual_value = int(actual_value)
            # Clamp to actual range
            actual_value = max(self.actual_from, min(self.actual_to, actual_value))
            # Convert to internal range
            internal_value = int((actual_value - self.actual_from) / self.stepsize)
            return max(self.internal_from, min(self.internal_to, internal_value))
        except (ValueError, TypeError):
            return self.internal_from

    def _internal_to_actual(self, internal_value):
        """Convert internal scale value to actual value"""
        try:
            internal_value = int(internal_value)
            # Clamp to internal range
            internal_value = max(self.internal_from, min(self.internal_to, internal_value))
            # Convert to actual range
            actual_value = self.actual_from + (internal_value * self.stepsize)
            return max(self.actual_from, min(self.actual_to, actual_value))
        except (ValueError, TypeError):
            return self.actual_from

    def get_actual_value(self):
        """Get the actual value (not internal scale value)"""
        internal_value = self.internal_variable.get()
        return self._internal_to_actual(internal_value)

    def set_actual_value(self, actual_value):
        """Set the actual value (not internal scale value)"""
        internal_value = self._actual_to_internal(actual_value)
        self.internal_variable.set(internal_value)

    def get(self, x=None, y=None):
        """Override get() to return actual value"""
        return self.get_actual_value()

    def set(self, value):
        """Override set() to accept actual value"""
        self.set_actual_value(value)


class LuminanceScale(CustomScale):
    """
    Specialized scale for luminance control with 10-step increments.

    This is a convenience class that pre-configures CustomScale for
    luminance values (0-100 with stepsize=10).
    """

    def __init__(self, parent, **kwargs):
        """
        Initialize LuminanceScale with default luminance settings.

        Args:
            parent: Parent widget
            **kwargs: Additional ttk.Scale arguments
        """
        # Set defaults for luminance control
        defaults = {"from_": 0, "to": 100, "stepsize": 10, "orient": tk.HORIZONTAL, "length": 300}

        # Merge with provided kwargs (kwargs take precedence)
        merged_kwargs = {**defaults, **kwargs}

        super().__init__(parent, **merged_kwargs)
