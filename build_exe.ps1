# Windows用のexeファイルの生成
uv run `
--group dev pyinstaller `
--name dimmer `
--icon="assets/icon-dark.ico" `
--add-data "assets/icon-dark.ico;assets/" `
--add-data "assets/icon-light.ico;assets/" `
--collect-data schedule `
--hidden-import=schedule `
--noconsole --onefile --clean `
main.py
