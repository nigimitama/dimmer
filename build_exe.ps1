# Windows用のexeファイルの生成
uv run --group dev pyinstaller --name dimmer --noconsole --onefile --clean --collect-data schedule main.py
