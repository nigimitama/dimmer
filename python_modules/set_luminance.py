import sys
from monitorcontrol import get_monitors


def set_luminances(value: int):
    for monitor in get_monitors():
        with monitor:
            try:
                old_value = monitor.get_luminance()
                monitor.set_luminance(value)
                print(f"set luminance: {old_value} -> {value}")
            except Exception:
                pass


if __name__ == '__main__':
    value = int(sys.argv[1])
    set_luminances(value)
