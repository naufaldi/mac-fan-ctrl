import "./app.css";
import { isTauri } from "@tauri-apps/api/core";
import { mount } from "svelte";
import App from "./App.svelte";

const appRoot = document.getElementById("app");

if (!appRoot) {
  throw new Error("App root element not found");
}

// Browser-only dev preview: install a Tauri IPC mock so the dashboard renders
// with sample data outside the native webview. Tree-shaken out of production
// builds (import.meta.env.DEV is statically false) and skipped inside Tauri.
if (import.meta.env.DEV && !isTauri()) {
  void (async () => {
    const { installDevMock } = await import("./lib/devMock");
    installDevMock();
    mount(App, { target: appRoot });
  })();
} else {
  mount(App, { target: appRoot });
}
