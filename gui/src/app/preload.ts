// See the Electron documentation for details on how to use preload scripts:
// https://www.electronjs.org/docs/latest/tutorial/process-model#preload-scripts

// TODO: type-safe IPC
import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("electron", {
  window: {
    minimize: () => ipcRenderer.invoke("window:minimize"),
    close: () => ipcRenderer.invoke("window:close"),
  },
});
