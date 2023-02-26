const { contextBridge, ipcRenderer } = require("electron");

// preloadはNode環境ではないのでmainとプロセス間連携する
contextBridge.exposeInMainWorld("monitorAPI", {
  getLuminance: () => ipcRenderer.invoke("getLuminance"),
  setLuminance: () => ipcRenderer.invoke("setLuminance"),
});
