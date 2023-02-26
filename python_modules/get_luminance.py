from monitorcontrol import get_monitors


def get_luminances():
    for monitor in get_monitors():
        with monitor:
            # nodeへは標準出力で
            print(monitor.get_luminance())


if __name__ == '__main__':
    get_luminances()
