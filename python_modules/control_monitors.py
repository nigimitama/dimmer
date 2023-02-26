from monitorcontrol import get_monitors


def get_luminances():
    for monitor in get_monitors():
        with monitor:
            # nodeへは標準出力で
            print(monitor.get_luminance())


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
    message = input()

    if message == 'get':
        get_luminances() # -> 標準出力

    if message == 'set':
        """次のような標準入力を想定
        set
        value
        """
        value = int(input())
        set_luminances(value)
