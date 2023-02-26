const { app, BrowserWindow, ipcMain } = require("electron");
const { getLuminance, setLuminance } = require("./monitor-controller.js");
const path = require("path");

const createWindow = () => {
  const win = new BrowserWindow({
    width: 800,
    height: 600,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
    },
  });
  ipcMain.handle("getLuminance", getLuminance);
  ipcMain.handle("setLuminance", async (_event, arg) => {
    await setLuminance(arg);
  });
  win.loadFile("index.html");
};

app.whenReady().then(() => {
  createWindow();

  // MacOSではウィンドウを閉じてもプロセスが残り続けるので判定を入れる
  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

// MacOSではウィンドウを閉じてもプロセスが残り続けるので判定を入れる
app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
